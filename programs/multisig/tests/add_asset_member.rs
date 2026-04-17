#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{AddAssetMemberInstructionArgs, CreateConfigProposalInstructionArgs},
    ConfigChange, Permissions, ProposalState,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, mark_config_proposal_as_stale, read_group, send_tx,
    set_config_proposal_as_expired, set_config_proposal_as_timelocked, set_config_proposal_state,
    setup_asset_mint, setup_group,
};

// Add asset member requires a passed proposal targeting the asset and a valid proposer.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        use_wrong_proposer: bool,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let new_member = group_setup.member_keys[3];
        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::AddAssetMember {
                member: new_member,
                weight: 1,
                permissions: Permissions::from_flags(true, true),
                asset_address: asset_setup.asset_address,
            },
        };

        let create_config_proposal = sdk::create_config_proposal(
            create_args,
            group_setup.group,
            group_setup.payer.pubkey(),
            Some(asset_setup.asset_address),
        );
        send_tx(svm, &group_setup.payer, vec![create_config_proposal], &[])?;
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let proposer_pubkey = if use_wrong_proposer {
            let wrong = solana_sdk::signature::Keypair::new();
            svm.airdrop(&wrong.pubkey(), 1_000_000_000)
                .expect("Could not airdrop to wrong proposer");
            wrong.pubkey()
        } else {
            group_setup.payer.pubkey()
        };

        let args = AddAssetMemberInstructionArgs { new_member };
        let add_asset_member = sdk::add_asset_member(
            args,
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            proposer_pubkey,
            group_setup.payer.pubkey(),
        );

        Ok((
            [add_asset_member],
            vec![group_setup.payer],
            group_setup.group,
        ))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        Self::builder(svm, false)
    }

    pub fn with_wrong_proposer(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (ix, signers, _) = Self::builder(svm, true)?;
        Ok((ix, signers))
    }

    /// Proposal is in the Open state (not yet Passed).
    pub fn with_proposal_not_passed(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let new_member = group_setup.member_keys[3];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::AddAssetMember {
                member: new_member,
                weight: 1,
                permissions: Permissions::from_flags(true, true),
                asset_address: asset_setup.asset_address,
            },
        };
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_config_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
                Some(asset_setup.asset_address),
            )],
            &[],
        )?;
        // Do NOT set state to Passed - proposal stays Open.

        let ix = sdk::add_asset_member(
            AddAssetMemberInstructionArgs { new_member },
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal is Passed but its deadline is in the past (expired).
    pub fn with_proposal_expired(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let new_member = group_setup.member_keys[3];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::AddAssetMember {
                member: new_member,
                weight: 1,
                permissions: Permissions::from_flags(true, true),
                asset_address: asset_setup.asset_address,
            },
        };
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_config_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
                Some(asset_setup.asset_address),
            )],
            &[],
        )?;
        set_config_proposal_as_expired(svm, proposal_pda.address)?;

        let ix = sdk::add_asset_member(
            AddAssetMemberInstructionArgs { new_member },
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal is Passed but its timelock keeps it locked in the far future.
    pub fn with_proposal_timelocked(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let new_member = group_setup.member_keys[3];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::AddAssetMember {
                member: new_member,
                weight: 1,
                permissions: Permissions::from_flags(true, true),
                asset_address: asset_setup.asset_address,
            },
        };
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_config_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
                Some(asset_setup.asset_address),
            )],
            &[],
        )?;
        set_config_proposal_as_timelocked(svm, proposal_pda.address)?;

        let ix = sdk::add_asset_member(
            AddAssetMemberInstructionArgs { new_member },
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal is Passed but stale (group's stale index was bumped past it).
    pub fn with_proposal_stale(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let new_member = group_setup.member_keys[3];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::AddAssetMember {
                member: new_member,
                weight: 1,
                permissions: Permissions::from_flags(true, true),
                asset_address: asset_setup.asset_address,
            },
        };
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_config_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
                Some(asset_setup.asset_address),
            )],
            &[],
        )?;
        mark_config_proposal_as_stale(svm, group_setup.group, proposal_pda.address)?;

        let ix = sdk::add_asset_member(
            AddAssetMemberInstructionArgs { new_member },
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal carries an AddGroupMember change instead of AddAssetMember.
    pub fn with_wrong_config_change(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let new_member = group_setup.member_keys[3];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            // Wrong change type: group-level member addition, not asset-level
            config_change: ConfigChange::AddGroupMember {
                member: new_member,
                weight: 1,
                permissions: Permissions::from_flags(true, true),
            },
        };
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_config_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
                None,
            )],
            &[],
        )?;
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let ix = sdk::add_asset_member(
            AddAssetMemberInstructionArgs { new_member },
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }
}

#[test]
fn test_add_asset_member_success() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_default(&mut svm);
    let (instructions, signers, group) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_transaction_success(result);

    let after = read_group(&svm, group).expect("read group after");
    assert_eq!(after.proposal_index_after_stale, after.next_proposal_index);
}

#[test]
fn test_add_asset_member_fails_with_wrong_proposer() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_proposer(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::InvalidProposer);
}

#[test]
fn test_add_asset_member_fails_when_proposal_not_passed() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_proposal_not_passed(&mut svm);
    let (instructions, signers) = match result {
        Ok(r) => r,
        Err(e) => panic!("setup failed: {e}"),
    };

    let payer = signers[0].pubkey();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer),
        &signers,
        svm.latest_blockhash(),
    );
    common::assert_multisig_instruction_error(
        svm.send_transaction(tx),
        0,
        multisig::MultisigError::ProposalNotPassed,
    );
}

#[test]
fn test_add_asset_member_fails_when_proposal_expired() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_proposal_expired(&mut svm);
    let (instructions, signers) = match result {
        Ok(r) => r,
        Err(e) => panic!("setup failed: {e}"),
    };

    let payer = signers[0].pubkey();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer),
        &signers,
        svm.latest_blockhash(),
    );
    common::assert_multisig_instruction_error(
        svm.send_transaction(tx),
        0,
        multisig::MultisigError::ProposalExpired,
    );
}

#[test]
fn test_add_asset_member_fails_when_proposal_timelocked() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_proposal_timelocked(&mut svm);
    let (instructions, signers) = match result {
        Ok(r) => r,
        Err(e) => panic!("setup failed: {e}"),
    };

    let payer = signers[0].pubkey();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer),
        &signers,
        svm.latest_blockhash(),
    );
    common::assert_multisig_instruction_error(
        svm.send_transaction(tx),
        0,
        multisig::MultisigError::ProposalStillTimelocked,
    );
}

#[test]
fn test_add_asset_member_fails_when_proposal_stale() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_proposal_stale(&mut svm);
    let (instructions, signers) = match result {
        Ok(r) => r,
        Err(e) => panic!("setup failed: {e}"),
    };

    let payer = signers[0].pubkey();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer),
        &signers,
        svm.latest_blockhash(),
    );
    common::assert_multisig_instruction_error(
        svm.send_transaction(tx),
        0,
        multisig::MultisigError::ProposalStale,
    );
}

#[test]
fn test_add_asset_member_fails_with_wrong_config_change() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_config_change(&mut svm);
    let (instructions, signers) = match result {
        Ok(r) => r,
        Err(e) => panic!("setup failed: {e}"),
    };

    let payer = signers[0].pubkey();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer),
        &signers,
        svm.latest_blockhash(),
    );
    common::assert_multisig_instruction_error(
        svm.send_transaction(tx),
        0,
        multisig::MultisigError::InvalidConfigChange,
    );
}

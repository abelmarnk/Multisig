#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::CreateConfigProposalInstructionArgs, ConfigChange, ConfigType, ProposalState,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, mark_config_proposal_as_stale, read_group, send_tx,
    set_config_proposal_as_expired, set_config_proposal_as_timelocked, set_config_proposal_state,
    setup_asset_mint, setup_group, threshold,
};

// Change asset config requires a passed proposal targeting the asset.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        mismatched_asset: bool,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeAssetConfig {
                config_type: ConfigType::Use(threshold(1, 2)),
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

        let asset_for_change = if mismatched_asset {
            let other_asset = setup_asset_mint(svm, &group_setup)?;
            other_asset.asset_address
        } else {
            asset_setup.asset_address
        };

        let change_asset_config = sdk::change_asset_config(
            group_setup.group,
            asset_for_change,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );

        Ok((
            [change_asset_config],
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

    pub fn with_mismatched_asset(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (ix, signers, _) = Self::builder(svm, true)?;
        Ok((ix, signers))
    }

    /// Build a successful change-asset-config instruction for any ConfigType.
    fn with_config_type_inner(
        svm: &mut LiteSVM,
        config_type: ConfigType,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeAssetConfig { config_type },
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
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let ix = sdk::change_asset_config(
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal is Passed but its deadline is in the past.
    pub fn with_proposal_expired(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeAssetConfig {
                config_type: ConfigType::MinimumVoteCount(2),
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

        let ix = sdk::change_asset_config(
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal is Passed but still timelocked.
    pub fn with_proposal_timelocked(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeAssetConfig {
                config_type: ConfigType::MinimumVoteCount(2),
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

        let ix = sdk::change_asset_config(
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal is Passed but stale.
    pub fn with_proposal_stale(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeAssetConfig {
                config_type: ConfigType::MinimumVoteCount(2),
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

        let ix = sdk::change_asset_config(
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Wrong proposer passed.
    pub fn with_wrong_proposer(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeAssetConfig {
                config_type: ConfigType::MinimumVoteCount(2),
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
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let wrong_proposer = solana_sdk::signature::Keypair::new().pubkey();
        let ix = sdk::change_asset_config(
            group_setup.group,
            asset_setup.asset_address,
            proposal_pda.address,
            wrong_proposer,
        );
        Ok(([ix], vec![group_setup.payer]))
    }
}

#[test]
fn test_change_asset_config_success() {
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
fn test_change_asset_config_fails_with_mismatched_asset() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_mismatched_asset(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::InvalidAsset);
}

/// All ConfigType variants must be accepted by change_asset_config.
#[test]
fn test_change_asset_config_all_config_types_succeed() {
    let all_config_types = [
        ConfigType::AddMember(threshold(1, 2)),
        ConfigType::NotAddMember(threshold(2, 3)),
        ConfigType::RemoveMember(threshold(1, 2)),
        ConfigType::NotRemoveMember(threshold(2, 3)),
        ConfigType::Use(threshold(1, 2)),
        ConfigType::NotUse(threshold(2, 3)),
        ConfigType::ChangeConfig(threshold(1, 2)),
        ConfigType::NotChangeConfig(threshold(2, 3)),
        ConfigType::MinimumMemberCount(2),
        ConfigType::MinimumVoteCount(2),
    ];

    for config_type in all_config_types {
        let mut svm = LiteSVM::new();
        add_multisig_program(&mut svm).expect("program load");

        let result = TestSetup::with_config_type_inner(&mut svm, config_type);
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
        common::assert_transaction_success(svm.send_transaction(tx));
    }
}

#[test]
fn test_change_asset_config_fails_when_proposal_expired() {
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
fn test_change_asset_config_fails_when_proposal_timelocked() {
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
fn test_change_asset_config_fails_when_proposal_stale() {
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
fn test_change_asset_config_fails_with_wrong_proposer() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_proposer(&mut svm);
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
        multisig::MultisigError::InvalidProposer,
    );
}

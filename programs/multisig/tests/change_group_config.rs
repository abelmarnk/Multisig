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
    set_config_proposal_as_expired, set_config_proposal_as_timelocked,
    set_config_proposal_config_change, set_config_proposal_state, setup_asset_mint, setup_group,
    threshold,
};

// Change group config requires a passed proposal with a group config change.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        use_wrong_config_change: bool,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let config_change = if use_wrong_config_change {
            ConfigChange::ChangeAssetConfig {
                config_type: ConfigType::Use(threshold(1, 2)),
            }
        } else {
            ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumVoteCount(1),
            }
        };

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change,
        };

        let asset = if use_wrong_config_change {
            Some(asset_setup.asset_address)
        } else {
            None
        };

        let create_config_proposal = sdk::create_config_proposal(
            create_args,
            group_setup.group,
            group_setup.payer.pubkey(),
            asset,
        );
        send_tx(svm, &group_setup.payer, vec![create_config_proposal], &[])?;
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let change_group_config = sdk::change_group_config(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );

        Ok((
            [change_group_config],
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

    pub fn with_wrong_config_change(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (ix, signers, _) = Self::builder(svm, true)?;
        Ok((ix, signers))
    }

    /// Build a successful change-group-config instruction for any valid ConfigType.
    fn with_config_type_inner(
        svm: &mut LiteSVM,
        config_type: ConfigType,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeGroupConfig { config_type },
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

        let ix = sdk::change_group_config(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal carries a Use ConfigType which is invalid for group config.
    /// Creates a valid proposal then patches the config_change via account injection.
    pub fn with_use_config_type(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumVoteCount(1),
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
        // Inject the invalid config type directly into the account.
        set_config_proposal_config_change(
            svm,
            proposal_pda.address,
            ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::Use(threshold(1, 2)),
            },
        )?;
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let ix = sdk::change_group_config(
            group_setup.group,
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

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumVoteCount(1),
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
        set_config_proposal_as_expired(svm, proposal_pda.address)?;

        let ix = sdk::change_group_config(
            group_setup.group,
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

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumVoteCount(1),
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
        set_config_proposal_as_timelocked(svm, proposal_pda.address)?;

        let ix = sdk::change_group_config(
            group_setup.group,
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

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumVoteCount(1),
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
        mark_config_proposal_as_stale(svm, group_setup.group, proposal_pda.address)?;

        let ix = sdk::change_group_config(
            group_setup.group,
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

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumVoteCount(1),
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

        let wrong_proposer = solana_sdk::signature::Keypair::new().pubkey();
        let ix = sdk::change_group_config(group_setup.group, proposal_pda.address, wrong_proposer);
        Ok(([ix], vec![group_setup.payer]))
    }
}

#[test]
fn test_change_group_config_success() {
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
fn test_change_group_config_fails_with_wrong_change() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_config_change(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(
        result,
        0,
        multisig::MultisigError::InvalidConfigChange,
    );
}

/// All valid group ConfigType variants must be accepted.
#[test]
fn test_change_group_config_all_valid_config_types_succeed() {
    let valid_config_types = [
        ConfigType::AddMember(threshold(1, 2)),
        ConfigType::NotAddMember(threshold(2, 3)),
        ConfigType::RemoveMember(threshold(1, 2)),
        ConfigType::NotRemoveMember(threshold(2, 3)),
        ConfigType::ChangeConfig(threshold(1, 2)),
        ConfigType::NotChangeConfig(threshold(2, 3)),
        ConfigType::MinimumMemberCount(2),
        ConfigType::MinimumVoteCount(2),
    ];

    for config_type in valid_config_types {
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
fn test_change_group_config_fails_with_use_config_type() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_use_config_type(&mut svm);
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
        multisig::MultisigError::UnexpectedConfigChange,
    );
}

#[test]
fn test_change_group_config_fails_when_proposal_expired() {
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
fn test_change_group_config_fails_when_proposal_timelocked() {
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
fn test_change_group_config_fails_when_proposal_stale() {
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
fn test_change_group_config_fails_with_wrong_proposer() {
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

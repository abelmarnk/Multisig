#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{instructions::CreateConfigProposalInstructionArgs, ConfigChange, ProposalState};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{add_multisig_program, send_tx, set_config_proposal_state, setup_group};

// Close config proposal should fail when proposal is still active.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        active_proposal: bool,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);
        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: multisig::ConfigType::MinimumVoteCount(1),
            },
        };
        let create_config_proposal = sdk::create_config_proposal(
            create_args,
            group_setup.group,
            group_setup.payer.pubkey(),
            None,
        );
        send_tx(svm, &group_setup.payer, vec![create_config_proposal], &[])?;

        if !active_proposal {
            set_config_proposal_state(svm, proposal_pda.address, ProposalState::Failed, None)?;
        }

        let close_config = sdk::close_config_proposal(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );

        Ok(([close_config], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, false)
    }

    pub fn with_active_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, true)
    }

    /// A wrong proposer key is passed - proposal.proposer does not match.
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
                config_type: multisig::ConfigType::MinimumVoteCount(1),
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
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Failed, None)?;

        // Pass a random key as proposer - does not match proposal.proposer.
        let wrong_proposer = solana_sdk::signature::Keypair::new().pubkey();
        let ix =
            sdk::close_config_proposal(group_setup.group, proposal_pda.address, wrong_proposer);
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal is in Passed state but not expired or stale -> still active.
    pub fn with_passed_but_active(
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
                config_type: multisig::ConfigType::MinimumVoteCount(1),
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
        // Set to Passed with ts=0; deadline=1000 is in the future relative to LiteSVM's clock=0.
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let ix = sdk::close_config_proposal(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal in Executed state can always be closed.
    pub fn with_executed_proposal(
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
                config_type: multisig::ConfigType::MinimumVoteCount(1),
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
        set_config_proposal_state(svm, proposal_pda.address, ProposalState::Executed, None)?;

        let ix = sdk::close_config_proposal(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }
}

#[test]
fn test_close_config_proposal_success() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_default(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_transaction_success(result);
}

#[test]
fn test_close_config_proposal_fails_when_active() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_active_proposal(&mut svm);
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
        multisig::MultisigError::ProposalStillActive,
    );
}

#[test]
fn test_close_config_proposal_fails_with_wrong_proposer() {
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

#[test]
fn test_close_config_proposal_fails_when_passed_and_still_active() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_passed_but_active(&mut svm);
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
        multisig::MultisigError::ProposalStillActive,
    );
}

#[test]
fn test_close_config_proposal_succeeds_when_executed() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_executed_proposal(&mut svm);
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

#![cfg(feature = "test-helpers")]
use anchor_lang::{AccountSerialize, Space};
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::CreateConfigProposalInstructionArgs, vote::VoteRecord, ConfigChange,
    ProposalState, VoteChoice,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, close_proposal_account, send_tx, set_config_proposal_deadline,
    set_config_proposal_state, set_group_stale_after_index, setup_group,
};

enum Scenario {
    Closed,
    Active,
    Failed,
    OpenExpired,
    Stale,
}

// Close config vote record should fail if proposal is still active.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        scenario: Scenario,
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

        // Set up state per scenario
        match &scenario {
            Scenario::Closed => {
                close_proposal_account(svm, proposal_pda.address)?;
            }
            Scenario::Failed => {
                set_config_proposal_state(svm, proposal_pda.address, ProposalState::Failed, None)?;
            }
            Scenario::OpenExpired => {
                // Proposal stays Open but deadline in the past -> is_expired=true
                set_config_proposal_deadline(svm, proposal_pda.address, -1)?;
            }
            Scenario::Stale => {
                // group.proposal_index_after_stale = MAX stales every proposal
                set_group_stale_after_index(svm, group_setup.group, u64::MAX)?;
            }
            Scenario::Active => {}
        }

        let vote_record_pda = sdk::config_vote_record_pda(
            &group_setup.group,
            &proposal_pda.address,
            &group_setup.payer.pubkey(),
        );
        let vote_record = VoteRecord::new(
            group_setup.payer.pubkey(),
            proposal_pda.address,
            None,
            vote_record_pda.bump,
            VoteChoice::For,
        );
        let mut data = Vec::with_capacity(8 + VoteRecord::INIT_SPACE);
        vote_record.try_serialize(&mut data)?;
        let rent = svm.minimum_balance_for_rent_exemption(data.len());
        let account = solana_sdk::account::Account {
            lamports: rent,
            data,
            owner: multisig::ID,
            executable: false,
            rent_epoch: 0,
        };
        svm.set_account(vote_record_pda.address, account)?;

        let close_vote_record = sdk::close_config_vote_record(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );

        Ok(([close_vote_record], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Closed)
    }

    pub fn with_active_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Active)
    }

    pub fn with_failed_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Failed)
    }

    pub fn with_open_expired_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::OpenExpired)
    }

    pub fn with_stale_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Stale)
    }
}

#[test]
fn test_close_config_vote_record_success() {
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
fn test_close_config_vote_record_fails_when_proposal_active() {
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
fn test_close_config_vote_record_succeeds_with_failed_proposal() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_failed_proposal(&mut svm);
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
fn test_close_config_vote_record_succeeds_with_expired_proposal() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_open_expired_proposal(&mut svm);
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
fn test_close_config_vote_record_succeeds_with_stale_proposal() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_stale_proposal(&mut svm);
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

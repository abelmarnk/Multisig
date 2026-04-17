#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{CreateConfigProposalInstructionArgs, VoteOnConfigProposalInstructionArgs},
    ConfigChange, ConfigType, ProposalState, VoteChoice,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, send_tx, set_config_proposal_deadline, set_config_proposal_state,
    set_group_member_weight, set_group_stale_after_index, setup_group,
};

// Vote on config proposal: open succeeds; various invalid states reject.
struct TestSetup {}

enum Scenario {
    Default,
    ProposalPassed,
    ExpiredProposal,
    StaleProposal,
    ZeroWeightVoter,
}

impl TestSetup {
    fn build_open_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(
        solana_sdk::pubkey::Pubkey,
        solana_sdk::pubkey::Pubkey,
        solana_sdk::signature::Keypair,
    )> {
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
        let create_config_proposal = sdk::create_config_proposal(
            create_args,
            group_setup.group,
            group_setup.payer.pubkey(),
            None,
        );
        send_tx(svm, &group_setup.payer, vec![create_config_proposal], &[])?;

        Ok((group_setup.group, proposal_pda.address, group_setup.payer))
    }

    fn builder(
        svm: &mut LiteSVM,
        scenario: Scenario,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (group, proposal, payer) = Self::build_open_proposal(svm)?;

        match scenario {
            Scenario::ProposalPassed => {
                set_config_proposal_state(svm, proposal, ProposalState::Passed, Some(0))?;
            }
            Scenario::ExpiredProposal => {
                // Keep proposal Open but push deadline to -1; LiteSVM clock = 0 -> expired.
                set_config_proposal_deadline(svm, proposal, -1)?;
            }
            Scenario::StaleProposal => {
                // Bump the group stale index past the proposal's index (0).
                set_group_stale_after_index(svm, group, u64::MAX)?;
            }
            Scenario::ZeroWeightVoter => {
                set_group_member_weight(svm, group, payer.pubkey(), 0)?;
            }
            Scenario::Default => {}
        }

        let vote_args = VoteOnConfigProposalInstructionArgs {
            vote: VoteChoice::For,
        };
        let vote_on_config =
            sdk::vote_on_config_proposal(vote_args, group, proposal, payer.pubkey(), None);

        Ok(([vote_on_config], vec![payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Default)
    }

    pub fn with_passed_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ProposalPassed)
    }

    pub fn with_expired_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ExpiredProposal)
    }

    pub fn with_stale_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::StaleProposal)
    }

    pub fn with_zero_weight_voter(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ZeroWeightVoter)
    }
}

#[test]
fn test_vote_on_config_proposal_success() {
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
fn test_vote_on_config_proposal_fails_when_not_open() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_passed_proposal(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::ProposalNotOpen);
}

#[test]
fn test_vote_on_config_proposal_fails_when_expired() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_expired_proposal(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::ProposalExpired);
}

#[test]
fn test_vote_on_config_proposal_fails_when_stale() {
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
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::ProposalStale);
}

#[test]
fn test_vote_on_config_proposal_fails_with_zero_weight_voter() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_zero_weight_voter(&mut svm);
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
        multisig::MultisigError::UnauthorizedVoter,
    );
}

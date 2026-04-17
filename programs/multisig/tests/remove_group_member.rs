#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{instructions::CreateConfigProposalInstructionArgs, ConfigChange, ProposalState};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, mark_config_proposal_as_stale, read_group, send_tx,
    set_config_proposal_as_expired, set_config_proposal_as_timelocked, set_config_proposal_state,
    setup_group,
};

// Remove group member requires a passed proposal and correct rent collector.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        set_passed: bool,
        rent_collector_override: Option<solana_sdk::pubkey::Pubkey>,
        proposer_override: Option<solana_sdk::pubkey::Pubkey>,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey, // group
        solana_sdk::pubkey::Pubkey, // proposal
    )> {
        let group_setup = setup_group(svm)?;
        let target_member = group_setup.member_keys[1];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);
        let create_args = CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            config_change: ConfigChange::RemoveGroupMember {
                member: target_member,
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
        if set_passed {
            set_config_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;
        }

        let rent_collector = rent_collector_override.unwrap_or_else(|| group_setup.payer.pubkey());
        let proposer = proposer_override.unwrap_or_else(|| group_setup.payer.pubkey());
        let ix = sdk::remove_group_member(
            group_setup.group,
            target_member,
            proposal_pda.address,
            rent_collector,
            proposer,
        );
        Ok((
            [ix],
            vec![group_setup.payer],
            group_setup.group,
            proposal_pda.address,
        ))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        let (ix, signers, group, _) = Self::builder(svm, true, None, None)?;
        Ok((ix, signers, group))
    }

    pub fn with_wrong_rent_collector(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let wrong = solana_sdk::signature::Keypair::new().pubkey();
        let (ix, signers, _, _) = Self::builder(svm, true, Some(wrong), None)?;
        Ok((ix, signers))
    }

    /// Wrong proposer key - `proposal.proposer` won't match.
    pub fn with_wrong_proposer(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let wrong = solana_sdk::signature::Keypair::new().pubkey();
        let (ix, signers, _, _) = Self::builder(svm, true, None, Some(wrong))?;
        Ok((ix, signers))
    }

    /// Proposal remains in Open state -> `ProposalNotPassed`.
    pub fn with_proposal_not_passed(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (ix, signers, _, _) = Self::builder(svm, false, None, None)?;
        Ok((ix, signers))
    }

    /// Proposal is Passed but deadline is in the past -> `ProposalExpired`.
    pub fn with_proposal_expired(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (ix, signers, _, proposal) = Self::builder(svm, false, None, None)?;
        set_config_proposal_as_expired(svm, proposal)?;
        Ok((ix, signers))
    }

    /// Proposal is Passed but timelock has not elapsed -> `ProposalStillTimelocked`.
    pub fn with_proposal_timelocked(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (ix, signers, _, proposal) = Self::builder(svm, false, None, None)?;
        set_config_proposal_as_timelocked(svm, proposal)?;
        Ok((ix, signers))
    }

    /// Group's `proposal_index_after_stale` is bumped past the proposal index -> `ProposalStale`.
    pub fn with_proposal_stale(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let (ix, signers, group, proposal) = Self::builder(svm, false, None, None)?;
        mark_config_proposal_as_stale(svm, group, proposal)?;
        Ok((ix, signers))
    }
}

#[test]
fn test_remove_group_member_success() {
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
fn test_remove_group_member_fails_with_wrong_rent_collector() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_rent_collector(&mut svm);
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
        multisig::MultisigError::UnexpectedRentCollector,
    );
}

#[test]
fn test_remove_group_member_fails_with_wrong_proposer() {
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
fn test_remove_group_member_fails_when_proposal_not_passed() {
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
fn test_remove_group_member_fails_when_proposal_expired() {
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
fn test_remove_group_member_fails_when_proposal_timelocked() {
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
fn test_remove_group_member_fails_when_proposal_stale() {
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

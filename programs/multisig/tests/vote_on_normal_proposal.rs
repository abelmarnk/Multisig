#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{
        CreateNormalProposalInstructionArgs, CreateProposalTransactionInstructionArgs,
        VoteOnNormalProposalInstructionArgs,
    },
    AssetIndex, ProposalState, SerializableInstruction, VoteChoice,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, create_token_account_at, send_tx, set_group_stale_after_index,
    set_normal_proposal_deadline, set_normal_proposal_state, setup_asset_mint, setup_group,
    to_serializable,
};

// Vote on normal proposal should reject invalid asset index.
struct TestSetup {}

enum Scenario {
    Default,
    InvalidAssetIndex,
    ProposalNotOpen,
    ExpiredProposal,
    StaleProposal,
}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        scenario: Scenario,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let destination = solana_sdk::signature::Keypair::new();
        create_token_account_at(
            svm,
            &destination.pubkey(),
            &asset_setup.mint,
            &group_setup.payer.pubkey(),
            solana_sdk::program_option::COption::None,
            spl_token::state::AccountState::Initialized,
            solana_sdk::program_option::COption::None,
        )?;

        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            &asset_setup.mint,
            &destination.pubkey(),
            &asset_setup.asset_authority,
            &[],
            1,
        )?;
        let serializable: SerializableInstruction = to_serializable(&mint_to_ix);
        let instruction_hashes = vec![sdk::serializable_instruction_hash(&serializable)?];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let create_normal_args = CreateNormalProposalInstructionArgs {
            proposal_seed,
            asset_keys: vec![asset_setup.mint],
            asset_indices: vec![AssetIndex {
                instruction_index: 0,
                account_index: 0,
            }],
            authority_bumps: vec![
                sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump,
            ],
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            instruction_hashes,
        };

        let create_normal_proposal = sdk::create_normal_proposal(
            create_normal_args,
            group_setup.group,
            group_setup.payer.pubkey(),
        );

        let proposal = sdk::proposal_pda(&group_setup.group, &proposal_seed).address;
        let raw_instructions = vec![sdk::serializable_instruction_bytes(&serializable)?];
        let create_tx_args = CreateProposalTransactionInstructionArgs { raw_instructions };
        let create_proposal_tx = sdk::create_proposal_transaction(
            create_tx_args,
            group_setup.group,
            proposal_seed,
            group_setup.payer.pubkey(),
            &[asset_setup.mint],
        );

        let voting_asset_index = match scenario {
            Scenario::InvalidAssetIndex => 5,
            _ => 0,
        };

        let vote_args = VoteOnNormalProposalInstructionArgs {
            voting_asset_index,
            vote: VoteChoice::For,
        };
        let vote_on_proposal = sdk::vote_on_normal_proposal(
            vote_args,
            group_setup.group,
            proposal,
            asset_setup.mint,
            group_setup.payer.pubkey(),
        );

        let setup_ixs = vec![create_normal_proposal, create_proposal_tx];
        send_tx(svm, &group_setup.payer, setup_ixs, &[])?;

        // Apply post-creation state mutations
        match scenario {
            Scenario::ProposalNotOpen => {
                set_normal_proposal_state(svm, proposal, ProposalState::Passed, Some(0))?;
            }
            Scenario::ExpiredProposal => {
                set_normal_proposal_deadline(svm, proposal, -1)?;
            }
            Scenario::StaleProposal => {
                set_group_stale_after_index(svm, group_setup.group, u64::MAX)?;
            }
            _ => {}
        }

        Ok((vec![vote_on_proposal], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Default)
    }

    pub fn with_invalid_asset_index(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::InvalidAssetIndex)
    }

    pub fn without_transaction_preimage(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        // Returns the vote instruction alone - no transaction PDA on-chain yet.
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let destination = solana_sdk::signature::Keypair::new();
        create_token_account_at(
            svm,
            &destination.pubkey(),
            &asset_setup.mint,
            &group_setup.payer.pubkey(),
            solana_sdk::program_option::COption::None,
            spl_token::state::AccountState::Initialized,
            solana_sdk::program_option::COption::None,
        )?;

        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            &asset_setup.mint,
            &destination.pubkey(),
            &asset_setup.asset_authority,
            &[],
            1,
        )?;
        let serializable: SerializableInstruction = to_serializable(&mint_to_ix);
        let instruction_hashes = vec![sdk::serializable_instruction_hash(&serializable)?];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let create_normal_args = CreateNormalProposalInstructionArgs {
            proposal_seed,
            asset_keys: vec![asset_setup.mint],
            asset_indices: vec![AssetIndex {
                instruction_index: 0,
                account_index: 0,
            }],
            authority_bumps: vec![
                sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump,
            ],
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            instruction_hashes,
        };

        let create_normal_proposal = sdk::create_normal_proposal(
            create_normal_args,
            group_setup.group,
            group_setup.payer.pubkey(),
        );
        send_tx(svm, &group_setup.payer, vec![create_normal_proposal], &[])?;

        let proposal = sdk::proposal_pda(&group_setup.group, &proposal_seed).address;
        let vote_args = VoteOnNormalProposalInstructionArgs {
            voting_asset_index: 0,
            vote: VoteChoice::For,
        };
        let vote_on_proposal = sdk::vote_on_normal_proposal(
            vote_args,
            group_setup.group,
            proposal,
            asset_setup.mint,
            group_setup.payer.pubkey(),
        );

        Ok((vec![vote_on_proposal], vec![group_setup.payer]))
    }

    pub fn with_proposal_not_open(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ProposalNotOpen)
    }

    pub fn with_expired_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ExpiredProposal)
    }

    pub fn with_stale_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::StaleProposal)
    }
}

#[test]
fn test_vote_on_normal_proposal_success() {
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
fn test_vote_on_normal_proposal_fails_with_invalid_asset_index() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_invalid_asset_index(&mut svm);
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
        multisig::MultisigError::InvalidAssetIndex,
    );
}

#[test]
fn test_vote_on_normal_proposal_fails_without_transaction_preimage() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::without_transaction_preimage(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);

    common::assert_instruction_error(
        result,
        0,
        u32::from(anchor_lang::error::ErrorCode::ConstraintOwner),
    );
}

#[test]
fn test_vote_on_normal_proposal_fails_when_not_open() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_proposal_not_open(&mut svm);
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
fn test_vote_on_normal_proposal_fails_when_expired() {
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
fn test_vote_on_normal_proposal_fails_when_stale() {
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

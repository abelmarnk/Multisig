#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{CreateNormalProposalInstructionArgs, CreateProposalTransactionInstructionArgs},
    AssetIndex, ProposalState, SerializableInstruction,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, create_token_account_at, send_tx, set_group_stale_after_index,
    set_normal_proposal_deadline, set_normal_proposal_state, setup_asset_mint, setup_group,
    to_serializable,
};

#[derive(Clone, Copy)]
enum Scenario {
    Default,
    WrongInstruction,
    ProposalNotOpen,
    StaleProposal,
    ExpiredProposal,
    NotEnoughAccountKeys,
    EmptyInstructions,
    LengthMismatch,
    WrongAuthorityBump,
}

// Create proposal transaction should validate instruction hash.
struct TestSetup {}

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
        let real_hash = sdk::serializable_instruction_hash(&serializable)?;

        // For NotEnoughAccountKeys: hash an instruction with 0 accounts
        let empty_ix = SerializableInstruction {
            program_id: spl_token::ID,
            accounts: vec![],
            data: vec![],
        };
        let empty_hash = sdk::serializable_instruction_hash(&empty_ix)?;

        let instruction_hashes = vec![match scenario {
            Scenario::NotEnoughAccountKeys => empty_hash,
            _ => real_hash,
        }];

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let create_normal_args = CreateNormalProposalInstructionArgs {
            proposal_seed,
            asset_keys: vec![asset_setup.mint],
            asset_indices: vec![AssetIndex {
                instruction_index: 0,
                account_index: 0,
            }],
            authority_bumps: vec![match scenario {
                Scenario::WrongAuthorityBump => {
                    sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint)
                        .bump
                        .wrapping_add(1)
                }
                _ => sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump,
            }],
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            instruction_hashes,
        };

        let create_normal_proposal = sdk::create_normal_proposal(
            create_normal_args.clone(),
            group_setup.group,
            group_setup.payer.pubkey(),
        );
        send_tx(svm, &group_setup.payer, vec![create_normal_proposal], &[])?;

        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        match &scenario {
            Scenario::ProposalNotOpen => {
                set_normal_proposal_state(
                    svm,
                    proposal_pda.address,
                    ProposalState::Passed,
                    Some(0),
                )?;
            }
            Scenario::StaleProposal => {
                set_group_stale_after_index(svm, group_setup.group, u64::MAX)?;
            }
            Scenario::ExpiredProposal => {
                set_normal_proposal_deadline(svm, proposal_pda.address, -1)?;
            }
            _ => {}
        }

        let raw_instructions = match scenario {
            Scenario::WrongInstruction => {
                let bad_ix = solana_sdk::system_instruction::transfer(
                    &group_setup.payer.pubkey(),
                    &destination.pubkey(),
                    1,
                );
                let bad_serializable = to_serializable(&bad_ix);
                vec![sdk::serializable_instruction_bytes(&bad_serializable)?]
            }
            Scenario::NotEnoughAccountKeys => vec![sdk::serializable_instruction_bytes(&empty_ix)?],
            Scenario::EmptyInstructions => vec![],
            Scenario::LengthMismatch => {
                let bytes = sdk::serializable_instruction_bytes(&serializable)?;
                vec![bytes.clone(), bytes]
            }
            _ => vec![sdk::serializable_instruction_bytes(&serializable)?],
        };

        let proposal_transaction_args =
            CreateProposalTransactionInstructionArgs { raw_instructions };
        let create_proposal_tx = sdk::create_proposal_transaction(
            proposal_transaction_args,
            group_setup.group,
            proposal_seed,
            group_setup.payer.pubkey(),
            &[asset_setup.mint],
        );

        Ok((vec![create_proposal_tx], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Default)
    }

    pub fn with_wrong_instruction(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::WrongInstruction)
    }

    pub fn with_proposal_not_open(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ProposalNotOpen)
    }

    pub fn with_stale_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::StaleProposal)
    }

    pub fn with_expired_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ExpiredProposal)
    }

    pub fn with_invalid_asset_index(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::NotEnoughAccountKeys)
    }

    pub fn with_empty_instructions(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::EmptyInstructions)
    }

    pub fn with_length_mismatch(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::LengthMismatch)
    }

    pub fn with_wrong_authority_bump(
        svm: &mut LiteSVM,
    ) -> Result<(Vec<Instruction>, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::WrongAuthorityBump)
    }
}

#[test]
fn test_create_proposal_transaction_success() {
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
fn test_create_proposal_transaction_fails_with_wrong_hash() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_instruction(&mut svm);
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
        multisig::MultisigError::InvalidInstructionHash,
    );
}

#[test]
fn test_create_proposal_transaction_fails_when_proposal_not_open() {
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
fn test_create_proposal_transaction_fails_when_stale() {
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
fn test_create_proposal_transaction_fails_when_expired() {
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
fn test_create_proposal_transaction_fails_with_invalid_asset_index() {
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
fn test_create_proposal_transaction_fails_with_empty_instructions() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_empty_instructions(&mut svm);
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
        multisig::MultisigError::EmptyInstructions,
    );
}

#[test]
fn test_create_proposal_transaction_fails_with_length_mismatch() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_length_mismatch(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::LengthMismatch);
}

#[test]
fn test_create_proposal_transaction_fails_with_wrong_authority_bump() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_authority_bump(&mut svm);
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

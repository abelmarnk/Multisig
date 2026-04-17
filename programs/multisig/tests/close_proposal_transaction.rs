#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{proposal::ProposalTransaction, AssetIndex, SerializableInstruction};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, close_proposal_account, insert_proposal_transaction, send_tx,
    setup_asset_mint, setup_group, to_serializable,
};

use multisig::instructions::{
    CreateNormalProposalInstructionArgs, CreateProposalTransactionInstructionArgs,
};

// Close proposal transaction requires the correct rent collector.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        wrong_rent_collector: bool,
        wrong_group: bool,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let other_group_setup = if wrong_group {
            Some(setup_group(svm)?)
        } else {
            None
        };

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            &asset_setup.mint,
            &group_setup.payer.pubkey(),
            &asset_setup.asset_authority,
            &[],
            1,
        )?;
        let serializable: SerializableInstruction = to_serializable(&mint_to_ix);

        let proposal_tx_pda = sdk::proposal_transaction_pda(&proposal_pda.address);
        let proposal_tx = ProposalTransaction::new(
            proposal_pda.address,
            group_setup.group,
            0,
            vec![AssetIndex {
                instruction_index: 0,
                account_index: 0,
            }],
            vec![[sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump]],
            vec![serializable],
            proposal_tx_pda.bump,
        );
        insert_proposal_transaction(svm, proposal_tx_pda.address, proposal_tx)?;

        close_proposal_account(svm, proposal_pda.address)?;

        let (group_for_close, rent_collector) =
            if let Some(other_group) = other_group_setup.as_ref() {
                (other_group.group, other_group.payer.pubkey())
            } else if wrong_rent_collector {
                (
                    group_setup.group,
                    solana_sdk::signature::Keypair::new().pubkey(),
                )
            } else {
                (group_setup.group, group_setup.payer.pubkey())
            };

        let close_proposal_tx = sdk::close_proposal_transaction(
            group_for_close,
            proposal_pda.address,
            proposal_tx_pda.address,
            rent_collector,
        );

        Ok(([close_proposal_tx], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, false, false)
    }

    pub fn with_wrong_rent_collector(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, true, false)
    }

    pub fn with_wrong_group(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, false, true)
    }

    /// Active proposal (Open, not expired/stale) -> ProposalStillActive
    pub fn with_active_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let destination = solana_sdk::signature::Keypair::new();
        common::create_token_account_at(
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
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);
        let proposal_tx_pda = sdk::proposal_transaction_pda(&proposal_pda.address);

        let create_args = CreateNormalProposalInstructionArgs {
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

        let create_normal =
            sdk::create_normal_proposal(create_args, group_setup.group, group_setup.payer.pubkey());
        let raw_instructions = vec![sdk::serializable_instruction_bytes(&serializable)?];
        let create_tx = sdk::create_proposal_transaction(
            CreateProposalTransactionInstructionArgs { raw_instructions },
            group_setup.group,
            proposal_seed,
            group_setup.payer.pubkey(),
            &[asset_setup.mint],
        );
        send_tx(svm, &group_setup.payer, vec![create_normal, create_tx], &[])?;

        // Proposal stays Open and within deadline -> ProposalStillActive
        let close_proposal_tx = sdk::close_proposal_transaction(
            group_setup.group,
            proposal_pda.address,
            proposal_tx_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([close_proposal_tx], vec![group_setup.payer]))
    }
}

#[test]
fn test_close_proposal_transaction_success() {
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
fn test_close_proposal_transaction_fails_with_wrong_rent_collector() {
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
fn test_close_proposal_transaction_fails_with_wrong_group() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_group(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::UnexpectedGroup);
}

#[test]
fn test_close_proposal_transaction_fails_with_active_proposal() {
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

#![cfg(feature = "test-helpers")]
use anchor_lang::{AccountSerialize, Space};
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::CreateNormalProposalInstructionArgs, vote::VoteRecord, AssetIndex, ProposalState,
    SerializableInstruction, VoteChoice,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, close_proposal_account, create_token_account_at, send_tx,
    set_group_stale_after_index, set_normal_proposal_deadline, set_normal_proposal_state,
    setup_asset_mint, setup_group, to_serializable,
};

enum Scenario {
    ClosedProposal,
    ActiveProposal,
    FailedProposal,
    OpenExpiredProposal,
    StaleProposal,
    WrongAssetIndex,
}

// Close normal vote record should fail if proposal is still active.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        scenario: Scenario,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let proposal_seed = solana_sdk::pubkey::Pubkey::new_unique();
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

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
        send_tx(svm, &group_setup.payer, vec![create_normal], &[])?;

        // Apply state mutation per scenario
        match &scenario {
            Scenario::ClosedProposal => {
                close_proposal_account(svm, proposal_pda.address)?;
            }
            Scenario::FailedProposal => {
                set_normal_proposal_state(svm, proposal_pda.address, ProposalState::Failed, None)?;
            }
            Scenario::OpenExpiredProposal => {
                set_normal_proposal_deadline(svm, proposal_pda.address, -1)?;
            }
            Scenario::StaleProposal => {
                set_group_stale_after_index(svm, group_setup.group, u64::MAX)?;
            }
            Scenario::ActiveProposal | Scenario::WrongAssetIndex => {}
        }

        // Vote record always uses asset_index=0
        let vote_record_pda = sdk::normal_vote_record_pda(
            &group_setup.group,
            &proposal_pda.address,
            &group_setup.payer.pubkey(),
            0,
        );
        let vote_record = VoteRecord::new(
            group_setup.payer.pubkey(),
            proposal_pda.address,
            Some(0),
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

        // For WrongAssetIndex we derive the close ix with asset_index=1 but vote record is for 0
        let close_asset_index = matches!(scenario, Scenario::WrongAssetIndex)
            .then_some(1u8)
            .unwrap_or(0u8);

        // When using wrong asset_index the PDA address will be different; to get a valid PDA we
        // must close using the SDK's derivation for that index but point to the real vote_record.
        // Instead: pass wrong u8 via a raw instruction override.
        // Actually the SDK simply uses asset_index in the seed. Using asset_index=1 means Anchor
        // will try to load a different PDA -> constraint mismatch, not InvalidAssetIndex.
        // We need to plant a vote_record at the index=1 PDA address, so the constraint passes but
        // the on-chain check fires.
        if matches!(scenario, Scenario::WrongAssetIndex) {
            // Close the proposal so the proposal validity check is skipped
            close_proposal_account(svm, proposal_pda.address)?;

            // Seed a vote_record at the asset_index=1 PDA, but with asset_index=0 stored
            let vr_pda_1 = sdk::normal_vote_record_pda(
                &group_setup.group,
                &proposal_pda.address,
                &group_setup.payer.pubkey(),
                1,
            );
            let vote_record_wrong = VoteRecord::new(
                group_setup.payer.pubkey(),
                proposal_pda.address,
                Some(0), // stored asset_index is 0, but we'll call close with asset_index=1
                vr_pda_1.bump,
                VoteChoice::For,
            );
            let mut data2 = Vec::with_capacity(8 + VoteRecord::INIT_SPACE);
            vote_record_wrong.try_serialize(&mut data2)?;
            let rent2 = svm.minimum_balance_for_rent_exemption(data2.len());
            let account2 = solana_sdk::account::Account {
                lamports: rent2,
                data: data2,
                owner: multisig::ID,
                executable: false,
                rent_epoch: 0,
            };
            svm.set_account(vr_pda_1.address, account2)?;
        }

        let close_vote_record = sdk::close_normal_vote_record(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
            close_asset_index,
        );

        Ok(([close_vote_record], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ClosedProposal)
    }

    pub fn with_active_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ActiveProposal)
    }

    pub fn with_failed_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::FailedProposal)
    }

    pub fn with_open_expired_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::OpenExpiredProposal)
    }

    pub fn with_stale_proposal(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::StaleProposal)
    }

    pub fn with_wrong_asset_index(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::WrongAssetIndex)
    }
}

#[test]
fn test_close_normal_vote_record_success() {
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
fn test_close_normal_vote_record_fails_when_proposal_active() {
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
fn test_close_normal_vote_record_succeeds_with_failed_proposal() {
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
fn test_close_normal_vote_record_succeeds_with_expired_proposal() {
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
fn test_close_normal_vote_record_succeeds_with_stale_proposal() {
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

#[test]
fn test_close_normal_vote_record_fails_with_wrong_asset_index() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_asset_index(&mut svm);
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

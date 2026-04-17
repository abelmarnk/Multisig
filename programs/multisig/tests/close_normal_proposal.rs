#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::CreateNormalProposalInstructionArgs, AssetIndex, ProposalState,
    SerializableInstruction,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, create_token_account_at, send_tx, set_normal_proposal_state,
    setup_asset_mint, setup_group, to_serializable,
};

// Close normal proposal should fail when proposal is still active.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        active_proposal: bool,
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

        if !active_proposal {
            set_normal_proposal_state(svm, proposal_pda.address, ProposalState::Failed, None)?;
        }

        let close_normal = sdk::close_normal_proposal(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );

        Ok(([close_normal], vec![group_setup.payer]))
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

    /// Wrong proposer key - `proposal.proposer` will not match the supplied key.
    pub fn with_wrong_proposer(
        svm: &mut LiteSVM,
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
        let serializable = to_serializable(&mint_to_ix);
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
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_normal_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
            )],
            &[],
        )?;
        set_normal_proposal_state(svm, proposal_pda.address, ProposalState::Failed, None)?;

        let wrong_proposer = solana_sdk::signature::Keypair::new().pubkey();
        let ix =
            sdk::close_normal_proposal(group_setup.group, proposal_pda.address, wrong_proposer);
        Ok(([ix], vec![group_setup.payer]))
    }

    /// Proposal in Passed state, deadline in future (clock=0 < 1000) - not expired or stale.
    pub fn with_passed_but_active(
        svm: &mut LiteSVM,
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
        let serializable = to_serializable(&mint_to_ix);
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
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_normal_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
            )],
            &[],
        )?;
        // Passed + deadline still in future -> is_expired=false, is_stale=false -> ProposalStillActive
        set_normal_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;

        let ix = sdk::close_normal_proposal(
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
        let serializable = to_serializable(&mint_to_ix);
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
        send_tx(
            svm,
            &group_setup.payer,
            vec![sdk::create_normal_proposal(
                create_args,
                group_setup.group,
                group_setup.payer.pubkey(),
            )],
            &[],
        )?;
        set_normal_proposal_state(svm, proposal_pda.address, ProposalState::Executed, None)?;

        let ix = sdk::close_normal_proposal(
            group_setup.group,
            proposal_pda.address,
            group_setup.payer.pubkey(),
        );
        Ok(([ix], vec![group_setup.payer]))
    }
}

#[test]
fn test_close_normal_proposal_success() {
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
fn test_close_normal_proposal_fails_when_active() {
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
fn test_close_normal_proposal_fails_with_wrong_proposer() {
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
fn test_close_normal_proposal_fails_when_passed_and_still_active() {
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
fn test_close_normal_proposal_succeeds_when_executed() {
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

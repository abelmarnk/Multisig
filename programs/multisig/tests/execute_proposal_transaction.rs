#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{CreateNormalProposalInstructionArgs, CreateProposalTransactionInstructionArgs},
    AssetIndex, ProposalState, SerializableInstruction,
};
use multisig_sdk as sdk;
use solana_sdk::{
    account::Account, instruction::AccountMeta, instruction::Instruction, signer::Signer,
    system_program, transaction::Transaction,
};

mod common;
use common::{
    add_multisig_program, create_token_account_at, send_tx, set_group_stale_after_index,
    set_normal_proposal_as_timelocked, set_normal_proposal_deadline, set_normal_proposal_state,
    setup_asset_mint, setup_group, to_serializable,
};

enum Scenario {
    Default,
    NotPassed,
    WrongRentCollector,
    Timelocked,
    StaleProposal,
    ExpiredProposal,
}

// Execute proposal transaction should require a passed proposal.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        scenario: Scenario,
    ) -> Result<(Instruction, Vec<solana_sdk::signature::Keypair>)> {
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
        let proposal_pda = sdk::proposal_pda(&group_setup.group, &proposal_seed);

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
        let create_tx_args = CreateProposalTransactionInstructionArgs { raw_instructions };
        let create_proposal_tx = sdk::create_proposal_transaction(
            create_tx_args,
            group_setup.group,
            proposal_seed,
            group_setup.payer.pubkey(),
            &[asset_setup.mint],
        );

        send_tx(
            svm,
            &group_setup.payer,
            vec![create_normal, create_proposal_tx],
            &[],
        )?;

        let proposal_tx_pda = sdk::proposal_transaction_pda(&proposal_pda.address);

        // Timelocked scenario sets Passed + timelock internally; others need explicit Passed.
        if !matches!(scenario, Scenario::NotPassed | Scenario::Timelocked) {
            set_normal_proposal_state(svm, proposal_pda.address, ProposalState::Passed, Some(0))?;
        }

        match &scenario {
            Scenario::Timelocked => {
                set_normal_proposal_as_timelocked(svm, proposal_pda.address)?;
            }
            Scenario::StaleProposal => {
                set_group_stale_after_index(svm, group_setup.group, u64::MAX)?;
            }
            Scenario::ExpiredProposal => {
                // deadline=-1 < now=0 -> ProposalExpired
                set_normal_proposal_deadline(svm, proposal_pda.address, -1)?;
            }
            _ => {}
        }

        let asset_authority_account = Account {
            lamports: 1,
            data: Vec::new(),
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        };
        svm.set_account(asset_setup.asset_authority, asset_authority_account)?;

        let remaining_accounts = vec![
            AccountMeta::new(asset_setup.mint, false),
            AccountMeta::new(destination.pubkey(), false),
            AccountMeta::new_readonly(asset_setup.asset_authority, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ];

        let rent_collector = match scenario {
            Scenario::WrongRentCollector => solana_sdk::pubkey::Pubkey::new_unique(),
            _ => group_setup.payer.pubkey(),
        };

        let execute = sdk::execute_proposal_transaction(
            group_setup.group,
            proposal_pda.address,
            proposal_tx_pda.address,
            rent_collector,
            remaining_accounts,
        );

        Ok((execute, vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<(Instruction, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Default)
    }

    pub fn with_unpassed_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Instruction, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::NotPassed)
    }

    pub fn with_wrong_rent_collector(
        svm: &mut LiteSVM,
    ) -> Result<(Instruction, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::WrongRentCollector)
    }

    pub fn with_timelocked_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Instruction, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Timelocked)
    }

    pub fn with_stale_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Instruction, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::StaleProposal)
    }

    pub fn with_expired_proposal(
        svm: &mut LiteSVM,
    ) -> Result<(Instruction, Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ExpiredProposal)
    }
}

#[test]
fn test_execute_proposal_transaction_success() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_default(&mut svm);
    let (instruction, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer),
        &signers,
        recent_blockhash,
    );

    let result = svm.send_transaction(transaction);
    common::assert_transaction_success(result);
}

#[test]
fn test_execute_proposal_transaction_fails_when_not_passed() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_unpassed_proposal(&mut svm);
    let (instruction, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer),
        &signers,
        recent_blockhash,
    );

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(
        result,
        0,
        multisig::MultisigError::ProposalNotPassed,
    );
}

#[test]
fn test_execute_proposal_transaction_fails_with_wrong_rent_collector() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_rent_collector(&mut svm);
    let (instruction, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer),
        &signers,
        recent_blockhash,
    );

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(
        result,
        0,
        multisig::MultisigError::UnexpectedRentCollector,
    );
}

#[test]
fn test_execute_proposal_transaction_fails_when_timelocked() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_timelocked_proposal(&mut svm);
    let (instruction, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer),
        &signers,
        recent_blockhash,
    );

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(
        result,
        0,
        multisig::MultisigError::ProposalStillTimelocked,
    );
}

#[test]
fn test_execute_proposal_transaction_fails_when_stale() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_stale_proposal(&mut svm);
    let (instruction, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer),
        &signers,
        recent_blockhash,
    );

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::ProposalStale);
}

#[test]
fn test_execute_proposal_transaction_fails_when_expired() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_expired_proposal(&mut svm);
    let (instruction, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer),
        &signers,
        recent_blockhash,
    );

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::ProposalExpired);
}

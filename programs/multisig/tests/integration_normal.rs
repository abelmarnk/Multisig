#![cfg(feature = "test-helpers")]

use litesvm::LiteSVM;
use multisig::{
    instructions::{
        CreateNormalProposalInstructionArgs, CreateProposalTransactionInstructionArgs,
        VoteOnNormalProposalInstructionArgs,
    },
    AssetIndex, SerializableInstruction, VoteChoice,
};
use multisig_sdk as sdk;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use std::path::Path;

mod common;
use common::{add_multisig_program, send_tx, setup_asset_mint, setup_group, to_serializable};

const TEST_HELPER_ID: Pubkey = solana_sdk::pubkey!("9uPtVeP3KVq1NqRtjpLLK6CKKjXwxS33k4HXtD5Snwjd");

fn add_test_helper_program(svm: &mut LiteSVM) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/multisig_test_helper.so");
    svm.add_program_from_file(TEST_HELPER_ID, &path)
        .expect("load multisig_test_helper.so");
}

fn build_test_helper_ix(
    asset_authority: Pubkey,
    asset_address: Pubkey,
    mint: Pubkey,
) -> SerializableInstruction {
    let ix = Instruction {
        program_id: TEST_HELPER_ID,
        accounts: vec![
            AccountMeta::new_readonly(asset_authority, true),
            AccountMeta::new_readonly(asset_address, false),
            AccountMeta::new_readonly(mint, false),
        ],
        data: vec![1],
    };
    to_serializable(&ix)
}

fn build_mint_to_ix(
    mint: Pubkey,
    destination: Pubkey,
    asset_authority: Pubkey,
) -> SerializableInstruction {
    let ix = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint,
        &destination,
        &asset_authority,
        &[],
        1,
    )
    .unwrap();
    to_serializable(&ix)
}

fn create_destination(
    svm: &mut LiteSVM,
    mint: Pubkey,
    owner: Pubkey,
) -> solana_sdk::signature::Keypair {
    let destination = solana_sdk::signature::Keypair::new();
    common::create_token_account_at(
        svm,
        &destination.pubkey(),
        &mint,
        &owner,
        solana_sdk::program_option::COption::None,
        spl_token::state::AccountState::Initialized,
        solana_sdk::program_option::COption::None,
    )
    .expect("create destination token account");
    destination
}

fn create_normal_with_transaction(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    asset_setup: &common::AssetSetup,
    instructions: &[SerializableInstruction],
) -> Pubkey {
    let proposal_seed = Pubkey::new_unique();
    let proposal = sdk::proposal_pda(&group_setup.group, &proposal_seed).address;
    let instruction_hashes =
        sdk::serializable_instruction_hashes(instructions).expect("hash instructions");

    let create_args = CreateNormalProposalInstructionArgs {
        proposal_seed,
        asset_keys: vec![asset_setup.mint],
        asset_indices: vec![AssetIndex {
            instruction_index: 0,
            account_index: 0,
        }],
        authority_bumps: vec![sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump],
        timelock_offset: 0,
        proposal_deadline_timestamp: 1000,
        instruction_hashes,
    };
    let create_normal =
        sdk::create_normal_proposal(create_args, group_setup.group, group_setup.payer.pubkey());

    let raw_instructions =
        sdk::serializable_instructions_bytes(instructions).expect("serialize instructions");
    let create_tx = sdk::create_proposal_transaction(
        CreateProposalTransactionInstructionArgs { raw_instructions },
        group_setup.group,
        proposal_seed,
        group_setup.payer.pubkey(),
        &[asset_setup.mint],
    );

    send_tx(svm, &group_setup.payer, vec![create_normal, create_tx], &[])
        .expect("create normal proposal and transaction");

    proposal
}

fn vote_normal(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    asset_setup: &common::AssetSetup,
    proposal: Pubkey,
    vote: VoteChoice,
) {
    let payer_vote = sdk::vote_on_normal_proposal(
        VoteOnNormalProposalInstructionArgs {
            voting_asset_index: 0,
            vote,
        },
        group_setup.group,
        proposal,
        asset_setup.mint,
        group_setup.payer.pubkey(),
    );
    let member_vote = sdk::vote_on_normal_proposal(
        VoteOnNormalProposalInstructionArgs {
            voting_asset_index: 0,
            vote,
        },
        group_setup.group,
        proposal,
        asset_setup.mint,
        group_setup.members[0].pubkey(),
    );

    send_tx(
        svm,
        &group_setup.payer,
        vec![payer_vote, member_vote],
        &[&group_setup.members[0]],
    )
    .expect("vote normal proposal");
}

fn fund_asset_authority(svm: &mut LiteSVM, asset_authority: Pubkey) {
    svm.set_account(
        asset_authority,
        Account {
            lamports: 1,
            data: Vec::new(),
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("set asset authority account");
}

fn execute_normal(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    asset_setup: &common::AssetSetup,
    proposal: Pubkey,
    destination: Pubkey,
    extra_remaining_accounts: Vec<AccountMeta>,
) {
    fund_asset_authority(svm, asset_setup.asset_authority);

    let proposal_tx = sdk::proposal_transaction_pda(&proposal).address;
    let mut remaining_accounts = vec![
        AccountMeta::new(asset_setup.mint, false),
        AccountMeta::new(destination, false),
        AccountMeta::new_readonly(asset_setup.asset_authority, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];
    remaining_accounts.extend(extra_remaining_accounts);

    let execute = sdk::execute_proposal_transaction(
        group_setup.group,
        proposal,
        proposal_tx,
        group_setup.payer.pubkey(),
        remaining_accounts,
    );
    send_tx(svm, &group_setup.payer, vec![execute], &[]).expect("execute proposal");
}

fn close_normal(svm: &mut LiteSVM, group_setup: &common::GroupSetup, proposal: Pubkey) {
    let close = sdk::close_normal_proposal(group_setup.group, proposal, group_setup.payer.pubkey());
    send_tx(svm, &group_setup.payer, vec![close], &[]).expect("close normal proposal");
}

#[test]
fn test_normal_proposal_full_cycle_single_instruction() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let group_setup = setup_group(&mut svm).expect("group setup");
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).expect("asset setup");
    let destination = create_destination(&mut svm, asset_setup.mint, group_setup.payer.pubkey());

    let mint_to = build_mint_to_ix(
        asset_setup.mint,
        destination.pubkey(),
        asset_setup.asset_authority,
    );
    let proposal = create_normal_with_transaction(&mut svm, &group_setup, &asset_setup, &[mint_to]);

    vote_normal(
        &mut svm,
        &group_setup,
        &asset_setup,
        proposal,
        VoteChoice::For,
    );
    execute_normal(
        &mut svm,
        &group_setup,
        &asset_setup,
        proposal,
        destination.pubkey(),
        Vec::new(),
    );
    close_normal(&mut svm, &group_setup, proposal);
}

#[test]
fn test_normal_proposal_full_cycle_multi_instruction() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("multisig load");
    add_test_helper_program(&mut svm);

    let group_setup = setup_group(&mut svm).expect("group setup");
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).expect("asset setup");
    let destination = create_destination(&mut svm, asset_setup.mint, group_setup.payer.pubkey());

    let mint_to = build_mint_to_ix(
        asset_setup.mint,
        destination.pubkey(),
        asset_setup.asset_authority,
    );
    let helper = build_test_helper_ix(
        asset_setup.asset_authority,
        asset_setup.asset_address,
        asset_setup.mint,
    );
    let proposal =
        create_normal_with_transaction(&mut svm, &group_setup, &asset_setup, &[mint_to, helper]);

    vote_normal(
        &mut svm,
        &group_setup,
        &asset_setup,
        proposal,
        VoteChoice::For,
    );
    execute_normal(
        &mut svm,
        &group_setup,
        &asset_setup,
        proposal,
        destination.pubkey(),
        vec![
            AccountMeta::new_readonly(TEST_HELPER_ID, false),
            AccountMeta::new_readonly(asset_setup.asset_address, false),
        ],
    );
}

#[test]
fn test_normal_proposal_rejects_mismatched_instruction_count() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let group_setup = setup_group(&mut svm).expect("group setup");
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).expect("asset setup");
    let destination = create_destination(&mut svm, asset_setup.mint, group_setup.payer.pubkey());
    let mint_to = build_mint_to_ix(
        asset_setup.mint,
        destination.pubkey(),
        asset_setup.asset_authority,
    );

    let proposal_seed = Pubkey::new_unique();
    let create_args = CreateNormalProposalInstructionArgs {
        proposal_seed,
        asset_keys: vec![asset_setup.mint],
        asset_indices: vec![AssetIndex {
            instruction_index: 0,
            account_index: 0,
        }],
        authority_bumps: vec![sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump],
        timelock_offset: 0,
        proposal_deadline_timestamp: 1000,
        instruction_hashes: vec![sdk::serializable_instruction_hash(&mint_to).unwrap()],
    };
    let create_normal =
        sdk::create_normal_proposal(create_args, group_setup.group, group_setup.payer.pubkey());

    let raw_bytes = sdk::serializable_instruction_bytes(&mint_to).unwrap();
    let create_tx = sdk::create_proposal_transaction(
        CreateProposalTransactionInstructionArgs {
            raw_instructions: vec![raw_bytes.clone(), raw_bytes],
        },
        group_setup.group,
        proposal_seed,
        group_setup.payer.pubkey(),
        &[asset_setup.mint],
    );

    let tx = Transaction::new_signed_with_payer(
        &[create_normal, create_tx],
        Some(&group_setup.payer.pubkey()),
        &[&group_setup.payer],
        svm.latest_blockhash(),
    );
    common::assert_multisig_instruction_error(
        svm.send_transaction(tx),
        1,
        multisig::MultisigError::LengthMismatch,
    );
}

#[test]
fn test_normal_proposal_declined_flow_can_be_closed() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let group_setup = setup_group(&mut svm).expect("group setup");
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).expect("asset setup");
    let destination = create_destination(&mut svm, asset_setup.mint, group_setup.payer.pubkey());
    let mint_to = build_mint_to_ix(
        asset_setup.mint,
        destination.pubkey(),
        asset_setup.asset_authority,
    );
    let proposal = create_normal_with_transaction(&mut svm, &group_setup, &asset_setup, &[mint_to]);

    vote_normal(
        &mut svm,
        &group_setup,
        &asset_setup,
        proposal,
        VoteChoice::Against,
    );
    close_normal(&mut svm, &group_setup, proposal);
}

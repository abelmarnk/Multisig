#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{CreateConfigProposalInstructionArgs, CreateNormalProposalInstructionArgs},
    ConfigChange, ConfigType, MultisigError,
};
use multisig_sdk as sdk;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

mod common;
use common::{
    add_multisig_program, assert_multisig_instruction_error, assert_transaction_success,
    setup_asset_mint, setup_group,
};

fn setup(svm: &mut LiteSVM) -> Result<()> {
    add_multisig_program(svm)?;
    Ok(())
}

#[test]
fn test_normal_proposal_fails_when_below_minimum_timelock() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();
    // minimum_timelock is 0 by default; bump it to 10 via a direct state write
    // so we can test the enforcement without a full config-proposal round-trip.
    common::utils::set_group_minimum_timelock(&mut svm, group_setup.group, 10).unwrap();
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).unwrap();

    let proposal_seed = Pubkey::new_unique();
    let ix = sdk::create_normal_proposal(
        CreateNormalProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 5, // below minimum_timelock=10
            proposal_deadline_timestamp: i64::MAX,
            instruction_hashes: vec![[0u8; 32]],
            asset_keys: vec![asset_setup.mint],
            authority_bumps: vec![
                sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump,
            ],
            asset_indices: vec![multisig::AssetIndex {
                instruction_index: 0,
                account_index: 0,
            }],
        },
        group_setup.group,
        group_setup.payer.pubkey(),
    );

    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[&group_setup.payer],
        svm.latest_blockhash(),
    ));

    assert_multisig_instruction_error(result, 0, MultisigError::TimelockBelowMinimum);
}

#[test]
fn test_normal_proposal_succeeds_at_minimum_timelock() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();
    common::utils::set_group_minimum_timelock(&mut svm, group_setup.group, 10).unwrap();
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).unwrap();

    let proposal_seed = Pubkey::new_unique();
    let ix = sdk::create_normal_proposal(
        CreateNormalProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 10, // exactly minimum_timelock
            proposal_deadline_timestamp: i64::MAX,
            instruction_hashes: vec![[0u8; 32]],
            asset_keys: vec![asset_setup.mint],
            authority_bumps: vec![
                sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump,
            ],
            asset_indices: vec![multisig::AssetIndex {
                instruction_index: 0,
                account_index: 0,
            }],
        },
        group_setup.group,
        group_setup.payer.pubkey(),
    );

    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[&group_setup.payer],
        svm.latest_blockhash(),
    ));

    assert_transaction_success(result);
}

#[test]
fn test_config_proposal_minimum_timelock_change_succeeds() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    let proposal_seed = Pubkey::new_unique();
    let ix = sdk::create_config_proposal(
        CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: i64::MAX,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumTimelock(30),
            },
        },
        group_setup.group,
        group_setup.payer.pubkey(),
        None,
    );

    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[&group_setup.payer],
        svm.latest_blockhash(),
    ));

    assert_transaction_success(result);
}

#[test]
fn test_config_proposal_fails_when_below_minimum_timelock() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();
    common::utils::set_group_minimum_timelock(&mut svm, group_setup.group, 60).unwrap();

    let proposal_seed = Pubkey::new_unique();
    let ix = sdk::create_config_proposal(
        CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0, // below minimum_timelock=60
            proposal_deadline_timestamp: i64::MAX,
            config_change: ConfigChange::ChangeGroupConfig {
                config_type: ConfigType::MinimumVoteCount(1),
            },
        },
        group_setup.group,
        group_setup.payer.pubkey(),
        None,
    );

    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[&group_setup.payer],
        svm.latest_blockhash(),
    ));

    assert_multisig_instruction_error(result, 0, MultisigError::TimelockBelowMinimum);
}

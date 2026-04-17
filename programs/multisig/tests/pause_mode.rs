#![cfg(feature = "test-helpers")]
//! Verifies that all normal operations are blocked while a group is in pause mode.
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{CreateConfigProposalInstructionArgs, CreateNormalProposalInstructionArgs},
    AssetIndex, ConfigChange, ConfigType, MultisigError,
};
use multisig_sdk as sdk;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

mod common;
use common::{
    add_multisig_program, assert_multisig_instruction_error, get_proposal, permissions, send_tx,
    setup_asset_mint, setup_group,
};

fn setup(svm: &mut LiteSVM) -> Result<()> {
    add_multisig_program(svm)?;
    Ok(())
}

/// Pause the group via direct state manipulation.
fn pause_group(svm: &mut LiteSVM, group: Pubkey, payer: Pubkey) {
    let t1 = Pubkey::new_unique();
    let t2 = Pubkey::new_unique();
    let t3 = Pubkey::new_unique();
    common::utils::set_group_paused(svm, group, true, t1, t2, t3).unwrap();
    // Store payer so the test can sign if needed - not needed here since we just
    // assert failures, but keep for symmetry.
    let _ = payer;
}

// ── normal proposal ──────────────────────────────────────────────────────────

#[test]
fn test_create_normal_proposal_fails_when_paused() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).unwrap();
    pause_group(&mut svm, group_setup.group, group_setup.payer.pubkey());

    let ix = sdk::create_normal_proposal(
        CreateNormalProposalInstructionArgs {
            proposal_seed: Pubkey::new_unique(),
            timelock_offset: 0,
            proposal_deadline_timestamp: i64::MAX,
            instruction_hashes: vec![[0u8; 32]],
            asset_keys: vec![asset_setup.mint],
            authority_bumps: vec![
                sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump,
            ],
            asset_indices: vec![AssetIndex {
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
    assert_multisig_instruction_error(result, 0, MultisigError::GroupPaused);
}

// ── config proposal ──────────────────────────────────────────────────────────

#[test]
fn test_create_config_proposal_fails_when_paused() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();
    pause_group(&mut svm, group_setup.group, group_setup.payer.pubkey());

    let ix = sdk::create_config_proposal(
        CreateConfigProposalInstructionArgs {
            proposal_seed: Pubkey::new_unique(),
            timelock_offset: 0,
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
    assert_multisig_instruction_error(result, 0, MultisigError::GroupPaused);
}

// ── add/remove group member (proposal-driven) ─────────────────────────────────

#[test]
fn test_add_group_member_fails_when_paused() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    // Create a (legitimately) passed add-member config proposal first, then pause
    let proposal_seed = Pubkey::new_unique();
    let new_member = Pubkey::new_unique();
    let create_ix = sdk::create_config_proposal(
        CreateConfigProposalInstructionArgs {
            proposal_seed,
            timelock_offset: 0,
            proposal_deadline_timestamp: i64::MAX,
            config_change: ConfigChange::AddGroupMember {
                member: new_member,
                weight: 1,
                permissions: permissions(),
            },
        },
        group_setup.group,
        group_setup.payer.pubkey(),
        None,
    );
    send_tx(&mut svm, &group_setup.payer, vec![create_ix], &[]).unwrap();

    let proposal_pda = get_proposal(&group_setup.group, &proposal_seed);
    let passed_ts = 0i64;
    common::utils::set_config_proposal_state(
        &mut svm,
        proposal_pda,
        multisig::ProposalState::Passed,
        Some(passed_ts),
    )
    .unwrap();

    // Now pause the group
    pause_group(&mut svm, group_setup.group, group_setup.payer.pubkey());

    let ix = sdk::add_group_member(
        multisig::instructions::AddGroupMemberInstructionArgs { new_member },
        group_setup.group,
        proposal_pda,
        group_setup.payer.pubkey(),
        group_setup.payer.pubkey(),
    );

    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[&group_setup.payer],
        svm.latest_blockhash(),
    ));
    assert_multisig_instruction_error(result, 0, MultisigError::GroupPaused);
}

// ── add member in reset mode fails without all trusted signers ────────────────

#[test]
fn test_add_member_in_reset_mode_fails_when_not_paused() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    // Group is NOT paused - the instruction must reject
    let t1 = group_setup.members[0].pubkey();
    let t2 = group_setup.members[1].pubkey();
    let t3 = group_setup.members[2].pubkey();

    let ix = sdk::add_member_in_reset_mode(
        multisig::instructions::AddMemberInResetModeArgs {
            new_member: Pubkey::new_unique(),
            weight: 1,
            permissions: permissions(),
        },
        group_setup.group,
        t1,
        t2,
        t3,
        group_setup.payer.pubkey(),
    );

    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[
            &group_setup.payer,
            &group_setup.members[0],
            &group_setup.members[1],
            &group_setup.members[2],
        ],
        svm.latest_blockhash(),
    ));
    assert_multisig_instruction_error(result, 0, MultisigError::GroupNotPaused);
}

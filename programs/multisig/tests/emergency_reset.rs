#![cfg(feature = "test-helpers")]
//! Tests for the emergency-reset proposal lifecycle and pause-mode instructions.
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::{
        AddMemberInResetModeArgs, CreateEmergencyResetProposalArgs, ExitPauseModeArgs,
        VoteOnEmergencyResetArgs,
    },
    proposal::ProposalState,
    MultisigError, VoteChoice,
};
use multisig_sdk as sdk;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

mod common;
use common::{
    add_multisig_program, assert_multisig_instruction_error, assert_transaction_success,
    get_emergency_reset_proposal, permissions, read_emergency_reset_proposal, read_group, send_tx,
    setup_group, threshold,
};

fn setup(svm: &mut LiteSVM) -> Result<()> {
    add_multisig_program(svm)?;
    Ok(())
}

fn vote_all_for(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    proposal: Pubkey,
) -> Result<()> {
    // Payer + 4 members must all vote For
    let voters: Vec<&solana_sdk::signature::Keypair> = std::iter::once(&group_setup.payer)
        .chain(group_setup.members.iter())
        .collect();

    for voter in voters {
        let ix = sdk::vote_on_emergency_reset_proposal(
            VoteOnEmergencyResetArgs {
                vote: VoteChoice::For,
            },
            group_setup.group,
            proposal,
            voter.pubkey(),
        );
        send_tx(svm, voter, vec![ix], &[])?;
    }
    Ok(())
}

#[test]
fn test_create_emergency_reset_proposal_succeeds() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    let proposal_seed = Pubkey::new_unique();
    let t1 = Pubkey::new_unique();
    let t2 = Pubkey::new_unique();
    let t3 = Pubkey::new_unique();
    let ix = sdk::create_emergency_reset_proposal(
        CreateEmergencyResetProposalArgs {
            proposal_seed,
            proposal_deadline_timestamp: i64::MAX,
            trusted_member_1: t1,
            trusted_member_2: t2,
            trusted_member_3: t3,
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

    let proposal_pda = get_emergency_reset_proposal(&group_setup.group, &proposal_seed);
    let proposal = read_emergency_reset_proposal(&svm, &proposal_pda).unwrap();
    assert!(matches!(proposal.state, ProposalState::Open));
    assert_eq!(proposal.trusted_members[0], t1);
    assert_eq!(proposal.trusted_members[1], t2);
    assert_eq!(proposal.trusted_members[2], t3);
}

#[test]
fn test_create_emergency_reset_proposal_fails_with_duplicate_trusted_members() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    let t1 = Pubkey::new_unique();
    let ix = sdk::create_emergency_reset_proposal(
        CreateEmergencyResetProposalArgs {
            proposal_seed: Pubkey::new_unique(),
            proposal_deadline_timestamp: i64::MAX,
            trusted_member_1: t1,
            trusted_member_2: t1, // duplicate
            trusted_member_3: Pubkey::new_unique(),
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
    assert_multisig_instruction_error(result, 0, MultisigError::TrustedMembersNotUnique);
}

#[test]
fn test_unanimous_for_vote_executes_and_pauses_group() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    let proposal_seed = Pubkey::new_unique();
    let t1 = Pubkey::new_unique();
    let t2 = Pubkey::new_unique();
    let t3 = Pubkey::new_unique();

    // Create
    let ix = sdk::create_emergency_reset_proposal(
        CreateEmergencyResetProposalArgs {
            proposal_seed,
            proposal_deadline_timestamp: i64::MAX,
            trusted_member_1: t1,
            trusted_member_2: t2,
            trusted_member_3: t3,
        },
        group_setup.group,
        group_setup.payer.pubkey(),
    );
    send_tx(&mut svm, &group_setup.payer, vec![ix], &[]).unwrap();

    let proposal_pda = get_emergency_reset_proposal(&group_setup.group, &proposal_seed);

    // Vote - all 5 members vote For
    vote_all_for(&mut svm, &group_setup, proposal_pda).unwrap();

    // Proposal must now be Passed
    let proposal = read_emergency_reset_proposal(&svm, &proposal_pda).unwrap();
    assert!(matches!(proposal.state, ProposalState::Passed));

    // Execute
    let ix =
        sdk::execute_emergency_reset(group_setup.group, proposal_pda, group_setup.payer.pubkey());
    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[&group_setup.payer],
        svm.latest_blockhash(),
    ));
    assert_transaction_success(result);

    // Group must now be paused with the three trusted keys stored
    let group = read_group(&svm, group_setup.group).unwrap();
    assert!(group.paused);
    assert_eq!(group.reset_trusted_1, t1);
    assert_eq!(group.reset_trusted_2, t2);
    assert_eq!(group.reset_trusted_3, t3);
}

#[test]
fn test_add_member_in_reset_mode_succeeds() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    let t1 = group_setup.members[0].pubkey();
    let t2 = group_setup.members[1].pubkey();
    let t3 = group_setup.members[2].pubkey();
    common::utils::set_group_paused(&mut svm, group_setup.group, true, t1, t2, t3).unwrap();

    let new_member = Pubkey::new_unique();

    let ix = sdk::add_member_in_reset_mode(
        AddMemberInResetModeArgs {
            new_member,
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
    assert_transaction_success(result);

    // member_count should have increased by 1 (was 5)
    let group = read_group(&svm, group_setup.group).unwrap();
    assert_eq!(group.member_count, 6);
}

#[test]
fn test_add_member_in_reset_mode_fails_with_wrong_trusted_signer() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    let t1 = group_setup.members[0].pubkey();
    let t2 = group_setup.members[1].pubkey();
    let t3 = group_setup.members[2].pubkey();
    common::utils::set_group_paused(&mut svm, group_setup.group, true, t1, t2, t3).unwrap();

    let imposter = group_setup.members[3].pubkey();
    let ix = sdk::add_member_in_reset_mode(
        AddMemberInResetModeArgs {
            new_member: Pubkey::new_unique(),
            weight: 1,
            permissions: permissions(),
        },
        group_setup.group,
        imposter, // wrong
        t2,
        t3,
        group_setup.payer.pubkey(),
    );

    let result = svm.send_transaction(solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&group_setup.payer.pubkey()),
        &[
            &group_setup.payer,
            &group_setup.members[3],
            &group_setup.members[1],
            &group_setup.members[2],
        ],
        svm.latest_blockhash(),
    ));
    assert_multisig_instruction_error(result, 0, MultisigError::InvalidTrustedMember);
}

#[test]
fn test_exit_pause_mode_succeeds() {
    let mut svm = LiteSVM::new();
    setup(&mut svm).unwrap();
    let group_setup = setup_group(&mut svm).unwrap();

    let t1 = group_setup.members[0].pubkey();
    let t2 = group_setup.members[1].pubkey();
    let t3 = group_setup.members[2].pubkey();
    common::utils::set_group_paused(&mut svm, group_setup.group, true, t1, t2, t3).unwrap();

    let ix = sdk::exit_pause_mode(
        ExitPauseModeArgs {
            add_threshold: threshold(1, 2),
            not_add_threshold: threshold(2, 3),
            remove_threshold: threshold(1, 2),
            not_remove_threshold: threshold(2, 3),
            change_config_threshold: threshold(1, 2),
            not_change_config_threshold: threshold(2, 3),
            minimum_member_count: 2,
            minimum_vote_count: 2,
            max_member_weight: 100,
            minimum_timelock: 0,
        },
        group_setup.group,
        t1,
        t2,
        t3,
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
    assert_transaction_success(result);

    let group = read_group(&svm, group_setup.group).unwrap();
    assert!(!group.paused);
    assert_eq!(group.reset_trusted_1, Pubkey::default());
    assert_eq!(group.reset_trusted_2, Pubkey::default());
    assert_eq!(group.reset_trusted_3, Pubkey::default());
}

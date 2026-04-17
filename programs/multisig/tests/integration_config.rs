#![cfg(feature = "test-helpers")]

use litesvm::LiteSVM;
use multisig::{
    instructions::{
        AddGroupMemberInstructionArgs, CreateConfigProposalInstructionArgs,
        VoteOnConfigProposalInstructionArgs,
    },
    ConfigChange, ConfigType, Permissions, VoteChoice,
};
use multisig_sdk as sdk;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

mod common;
use common::{add_multisig_program, read_group, send_tx, setup_asset_mint, setup_group, threshold};

fn create_config_proposal(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    config_change: ConfigChange,
    asset_address: Option<Pubkey>,
) -> Pubkey {
    let proposal_seed = Pubkey::new_unique();
    let proposal = sdk::proposal_pda(&group_setup.group, &proposal_seed).address;
    let create_args = CreateConfigProposalInstructionArgs {
        proposal_seed,
        timelock_offset: 0,
        proposal_deadline_timestamp: 1000,
        config_change,
    };
    let create_ix = sdk::create_config_proposal(
        create_args,
        group_setup.group,
        group_setup.payer.pubkey(),
        asset_address,
    );
    send_tx(svm, &group_setup.payer, vec![create_ix], &[]).expect("create config proposal");
    proposal
}

fn vote_config_group(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    proposal: Pubkey,
    vote: VoteChoice,
) {
    let payer_vote = sdk::vote_on_config_proposal(
        VoteOnConfigProposalInstructionArgs { vote },
        group_setup.group,
        proposal,
        group_setup.payer.pubkey(),
        None,
    );
    let member_vote = sdk::vote_on_config_proposal(
        VoteOnConfigProposalInstructionArgs { vote },
        group_setup.group,
        proposal,
        group_setup.members[0].pubkey(),
        None,
    );
    send_tx(
        svm,
        &group_setup.payer,
        vec![payer_vote, member_vote],
        &[&group_setup.members[0]],
    )
    .expect("vote group config proposal");
}

fn vote_config_asset(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    proposal: Pubkey,
    asset_address: Pubkey,
    vote: VoteChoice,
) {
    let payer_vote = sdk::vote_on_config_proposal(
        VoteOnConfigProposalInstructionArgs { vote },
        group_setup.group,
        proposal,
        group_setup.payer.pubkey(),
        Some(asset_address),
    );
    let member_vote = sdk::vote_on_config_proposal(
        VoteOnConfigProposalInstructionArgs { vote },
        group_setup.group,
        proposal,
        group_setup.members[0].pubkey(),
        Some(asset_address),
    );
    send_tx(
        svm,
        &group_setup.payer,
        vec![payer_vote, member_vote],
        &[&group_setup.members[0]],
    )
    .expect("vote asset config proposal");
}

fn execute_add_group_member(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    proposal: Pubkey,
    new_member: Pubkey,
) {
    let ix = sdk::add_group_member(
        AddGroupMemberInstructionArgs { new_member },
        group_setup.group,
        proposal,
        group_setup.payer.pubkey(),
        group_setup.payer.pubkey(),
    );
    send_tx(svm, &group_setup.payer, vec![ix], &[]).expect("execute add group member");
}

fn execute_change_group_config(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    proposal: Pubkey,
) {
    let ix = sdk::change_group_config(group_setup.group, proposal, group_setup.payer.pubkey());
    send_tx(svm, &group_setup.payer, vec![ix], &[]).expect("execute group config change");
}

fn execute_change_asset_config(
    svm: &mut LiteSVM,
    group_setup: &common::GroupSetup,
    asset_address: Pubkey,
    proposal: Pubkey,
) {
    let ix = sdk::change_asset_config(
        group_setup.group,
        asset_address,
        proposal,
        group_setup.payer.pubkey(),
    );
    send_tx(svm, &group_setup.payer, vec![ix], &[]).expect("execute asset config change");
}

fn close_config(svm: &mut LiteSVM, group_setup: &common::GroupSetup, proposal: Pubkey) {
    let close = sdk::close_config_proposal(group_setup.group, proposal, group_setup.payer.pubkey());
    send_tx(svm, &group_setup.payer, vec![close], &[]).expect("close config proposal");
}

#[test]
fn test_config_cycle_add_group_member() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let group_setup = setup_group(&mut svm).expect("group setup");
    let before = read_group(&svm, group_setup.group).expect("read group before");
    let new_member = solana_sdk::signature::Keypair::new();

    let proposal = create_config_proposal(
        &mut svm,
        &group_setup,
        ConfigChange::AddGroupMember {
            member: new_member.pubkey(),
            weight: 1,
            permissions: Permissions::from_flags(true, true),
        },
        None,
    );
    vote_config_group(&mut svm, &group_setup, proposal, VoteChoice::For);
    execute_add_group_member(&mut svm, &group_setup, proposal, new_member.pubkey());

    let after = read_group(&svm, group_setup.group).expect("read group after");
    assert_eq!(after.member_count, before.member_count + 1);
    assert_eq!(after.proposal_index_after_stale, after.next_proposal_index);
}

#[test]
fn test_config_cycle_three_group_config_changes() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let group_setup = setup_group(&mut svm).expect("group setup");
    let changes = [
        ConfigType::MinimumVoteCount(2),
        ConfigType::MinimumMemberCount(2),
        ConfigType::ChangeConfig(threshold(1, 2)),
    ];

    for config_type in changes {
        let proposal = create_config_proposal(
            &mut svm,
            &group_setup,
            ConfigChange::ChangeGroupConfig { config_type },
            None,
        );
        vote_config_group(&mut svm, &group_setup, proposal, VoteChoice::For);
        execute_change_group_config(&mut svm, &group_setup, proposal);
    }

    let group = read_group(&svm, group_setup.group).expect("read group");
    assert_eq!(group.proposal_index_after_stale, group.next_proposal_index);
}

#[test]
fn test_config_cycle_three_asset_config_changes() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let group_setup = setup_group(&mut svm).expect("group setup");
    let asset_setup = setup_asset_mint(&mut svm, &group_setup).expect("asset setup");
    let changes = [
        ConfigType::Use(threshold(1, 2)),
        ConfigType::AddMember(threshold(1, 2)),
        ConfigType::RemoveMember(threshold(1, 2)),
    ];

    for config_type in changes {
        let proposal = create_config_proposal(
            &mut svm,
            &group_setup,
            ConfigChange::ChangeAssetConfig { config_type },
            Some(asset_setup.asset_address),
        );
        vote_config_asset(
            &mut svm,
            &group_setup,
            proposal,
            asset_setup.asset_address,
            VoteChoice::For,
        );
        execute_change_asset_config(&mut svm, &group_setup, asset_setup.asset_address, proposal);
    }

    let group = read_group(&svm, group_setup.group).expect("read group");
    assert_eq!(group.proposal_index_after_stale, group.next_proposal_index);
}

#[test]
fn test_config_declined_proposal_can_be_closed() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let group_setup = setup_group(&mut svm).expect("group setup");
    let new_member = solana_sdk::signature::Keypair::new();
    let proposal = create_config_proposal(
        &mut svm,
        &group_setup,
        ConfigChange::AddGroupMember {
            member: new_member.pubkey(),
            weight: 1,
            permissions: Permissions::from_flags(true, true),
        },
        None,
    );

    vote_config_group(&mut svm, &group_setup, proposal, VoteChoice::Against);
    close_config(&mut svm, &group_setup, proposal);
}

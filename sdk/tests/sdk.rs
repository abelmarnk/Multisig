use multisig::{
    instructions::{
        CreateConfigProposalInstructionArgs, CreateGroupInstructionArgs,
        CreateNormalProposalInstructionArgs, CreateProposalTransactionInstructionArgs,
        VoteOnConfigProposalInstructionArgs, VoteOnNormalProposalInstructionArgs,
    },
    AssetIndex, ConfigChange, ConfigType, FractionalThreshold, Permissions, VoteChoice,
};
use solana_sdk::{hash, pubkey::Pubkey, system_program};

fn threshold(numerator: u32, denominator: u32) -> FractionalThreshold {
    FractionalThreshold::new_from_values(numerator, denominator).unwrap()
}

fn permissions() -> Permissions {
    Permissions::try_from(0b0000_0011).unwrap()
}

#[test]
fn derives_expected_pdas() {
    let group_seed = Pubkey::new_unique();
    let member = Pubkey::new_unique();
    let asset = Pubkey::new_unique();

    let group = multisig_sdk::group_pda(&group_seed);
    assert_eq!(
        Pubkey::find_program_address(&[b"group", group_seed.as_ref()], &multisig_sdk::PROGRAM_ID),
        (group.address, group.bump)
    );

    let asset_member = multisig_sdk::asset_member_pda(&group.address, &asset, &member);
    assert_eq!(
        Pubkey::find_program_address(
            &[
                b"asset-member",
                group.address.as_ref(),
                asset.as_ref(),
                member.as_ref()
            ],
            &multisig_sdk::PROGRAM_ID
        ),
        (asset_member.address, asset_member.bump)
    );
}

#[test]
fn builds_create_group_instruction_accounts() {
    let payer = Pubkey::new_unique();
    let members = [
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    ];
    let args = CreateGroupInstructionArgs {
        group_seed: Pubkey::new_unique(),
        rent_collector: Pubkey::new_unique(),
        add_threshold: threshold(1, 2),
        not_add_threshold: threshold(2, 3),
        remove_threshold: threshold(1, 2),
        not_remove_threshold: threshold(2, 3),
        change_config_threshold: threshold(1, 2),
        not_change_config_threshold: threshold(2, 3),
        minimum_member_count: 5,
        minimum_vote_count: 3,
        max_member_weight: 100,
        minimum_timelock: 0,
        member_weights: [20; 5],
        member_permissions: [permissions(); 5],
    };

    let group = multisig_sdk::group_pda(&args.group_seed).address;
    let ix = multisig_sdk::create_group(args, payer, members);

    assert_eq!(ix.program_id, multisig_sdk::PROGRAM_ID);
    assert_eq!(ix.accounts[0].pubkey, group);
    assert!(ix.accounts[0].is_writable);
    assert_eq!(ix.accounts[11].pubkey, payer);
    assert!(ix.accounts[11].is_signer);
    assert_eq!(ix.accounts[12].pubkey, system_program::ID);
    assert!(!ix.data.is_empty());
}

#[test]
fn create_normal_proposal_does_not_attach_registered_asset_accounts() {
    let group = Pubkey::new_unique();
    let proposer = Pubkey::new_unique();
    let asset = Pubkey::new_unique();
    let args = CreateNormalProposalInstructionArgs {
        proposal_seed: Pubkey::new_unique(),
        asset_keys: vec![asset],
        asset_indices: vec![AssetIndex {
            instruction_index: 0,
            account_index: 0,
        }],
        authority_bumps: vec![multisig_sdk::asset_authority_pda(&group, &asset).bump],
        timelock_offset: 0,
        proposal_deadline_timestamp: 100,
        instruction_hashes: vec![hash::Hash::new_unique().to_bytes()],
    };

    let ix = multisig_sdk::create_normal_proposal(args, group, proposer);

    assert_eq!(ix.accounts.len(), 5);
}

#[test]
fn create_proposal_transaction_includes_registered_asset_accounts_as_remaining_accounts() {
    let group = Pubkey::new_unique();
    let payer = Pubkey::new_unique();
    let proposal_seed = Pubkey::new_unique();
    let asset = Pubkey::new_unique();
    let args = CreateProposalTransactionInstructionArgs {
        raw_instructions: vec![vec![1, 2, 3]],
    };

    let ix = multisig_sdk::create_proposal_transaction(args, group, proposal_seed, payer, &[asset]);

    assert_eq!(ix.accounts.len(), 6);
    assert_eq!(
        ix.accounts[5].pubkey,
        multisig_sdk::asset_pda(&group, &asset).address
    );
    assert!(!ix.accounts[5].is_writable);
}

#[test]
fn vote_on_normal_proposal_includes_preimage_transaction_account() {
    let group = Pubkey::new_unique();
    let proposal = Pubkey::new_unique();
    let asset = Pubkey::new_unique();
    let voter = Pubkey::new_unique();
    let args = VoteOnNormalProposalInstructionArgs {
        voting_asset_index: 0,
        vote: VoteChoice::For,
    };

    let ix = multisig_sdk::vote_on_normal_proposal(args, group, proposal, asset, voter);

    assert_eq!(
        ix.accounts[2].pubkey,
        multisig_sdk::proposal_transaction_pda(&proposal).address
    );
    assert!(!ix.accounts[2].is_writable);
}

#[test]
fn optional_sdk_accounts_use_readonly_program_id_sentinel() {
    let group = Pubkey::new_unique();
    let proposer = Pubkey::new_unique();
    let proposal_seed = Pubkey::new_unique();
    let create_args = CreateConfigProposalInstructionArgs {
        proposal_seed,
        timelock_offset: 0,
        proposal_deadline_timestamp: 100,
        config_change: ConfigChange::ChangeGroupConfig {
            config_type: ConfigType::MinimumVoteCount(1),
        },
    };

    let create_ix = multisig_sdk::create_config_proposal(create_args, group, proposer, None);
    assert_eq!(create_ix.accounts[2].pubkey, multisig::ID);
    assert!(!create_ix.accounts[2].is_writable);

    let vote_args = VoteOnConfigProposalInstructionArgs {
        vote: VoteChoice::For,
    };
    let proposal = multisig_sdk::proposal_pda(&group, &proposal_seed).address;
    let vote_ix = multisig_sdk::vote_on_config_proposal(vote_args, group, proposal, proposer, None);
    assert_eq!(vote_ix.accounts[2].pubkey, multisig::ID);
    assert!(!vote_ix.accounts[2].is_writable);
    assert_eq!(vote_ix.accounts[4].pubkey, multisig::ID);
    assert!(!vote_ix.accounts[4].is_writable);
}

use multisig::{
    Asset, FractionalThreshold, Group, GroupMember, Permissions, ProposalAsset,
    ProposalAssetThresholdState,
};
use solana_sdk::pubkey::Pubkey;

fn assert_anchor_error<T>(
    result: Result<T, anchor_lang::error::Error>,
    expected_name: &str,
    expected_code: u32,
    expected_msg: &str,
) {
    match result.err().expect("expected an error") {
        anchor_lang::error::Error::AnchorError(error) => {
            assert_eq!(error.error_name, expected_name);
            assert_eq!(error.error_code_number, expected_code);
            assert_eq!(error.error_msg, expected_msg);
        }
        other => panic!("expected AnchorError, got {other:?}"),
    }
}

#[test]
fn fractional_threshold_compares_without_division() {
    let threshold = FractionalThreshold::new_from_values(2, 3).unwrap();

    assert!(threshold.less_than_or_equal(4, 6).unwrap());
    assert!(threshold.less_than_or_equal(5, 6).unwrap());
    assert!(!threshold.less_than_or_equal(3, 6).unwrap());
    assert_eq!(threshold.numerator, 2);
    assert_eq!(threshold.denominator, 3);
}

#[test]
fn fractional_threshold_rejects_overlapping_pairs_and_allows_unanimity() {
    let pass = FractionalThreshold::new_from_values(1, 2).unwrap();
    let overlapping_fail = FractionalThreshold::new_from_values(1, 3).unwrap();
    assert_anchor_error(
        FractionalThreshold::validate_non_overlapping_pair(pass, overlapping_fail),
        "InvalidThreshold",
        6036,
        "Invalid threshold configuration",
    );

    let non_overlapping_fail = FractionalThreshold::new_from_values(2, 3).unwrap();
    assert!(FractionalThreshold::validate_non_overlapping_pair(pass, non_overlapping_fail).is_ok());

    let unanimity = FractionalThreshold::new_from_values(1, 1).unwrap();
    assert!(FractionalThreshold::validate_non_overlapping_pair(unanimity, pass).is_ok());
}

#[test]
fn permissions_reject_unknown_bits() {
    assert!(Permissions::try_from(0b0000_0011).is_ok());
    assert_anchor_error(
        Permissions::try_from(0b0000_0100),
        "InvalidPermissions",
        6051,
        "Invalid permissions",
    );
}

#[test]
fn member_constructors_reject_zero_and_oversized_weight() {
    let user = Pubkey::new_unique();
    let group = Pubkey::new_unique();
    let permissions = Permissions::try_from(0b0000_0011).unwrap();

    assert_anchor_error(
        GroupMember::new(user, group, permissions, 0, 255, 100),
        "InvalidMemberWeight",
        6046,
        "Invalid member weight",
    );
    assert_anchor_error(
        GroupMember::new(user, group, permissions, 101, 255, 100),
        "InvalidMemberWeight",
        6046,
        "Invalid member weight",
    );

    let asset = Pubkey::new_unique();
    assert_anchor_error(
        multisig::AssetMember::new(user, group, asset, permissions, 0, 255, 100),
        "InvalidMemberWeight",
        6046,
        "Invalid member weight",
    );
    assert_anchor_error(
        multisig::AssetMember::new(user, group, asset, permissions, 101, 255, 100),
        "InvalidMemberWeight",
        6046,
        "Invalid member weight",
    );
}

#[test]
fn group_and_asset_quorum_counts_are_positive_and_can_equal_member_count() {
    let pass = FractionalThreshold::new_from_values(1, 2).unwrap();
    let fail = FractionalThreshold::new_from_values(2, 3).unwrap();

    assert_anchor_error(
        Group::new(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            pass,
            fail,
            pass,
            fail,
            pass,
            fail,
            1,
            0,
            100,
            0,
            5,
            255,
        ),
        "InvalidMemberCount",
        6045,
        "Invalid member count",
    );

    let mut group = Group::new(
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        pass,
        fail,
        pass,
        fail,
        pass,
        fail,
        5,
        5,
        100,
        0,
        5,
        255,
    )
    .unwrap();
    assert_anchor_error(
        group.set_minimum_vote_count(0),
        "InvalidMemberCount",
        6045,
        "Invalid member count",
    );
    assert!(group.set_minimum_vote_count(5).is_ok());

    let mut asset = Asset::new(
        Pubkey::new_unique(),
        pass,
        fail,
        pass,
        fail,
        pass,
        fail,
        pass,
        fail,
        3,
        3,
        3,
        254,
        253,
    )
    .unwrap();
    assert_anchor_error(
        asset.set_minimum_vote_count(1),
        "InvalidThreshold",
        6036,
        "Invalid threshold configuration",
    );
    assert!(asset.set_minimum_vote_count(3).is_ok());
}

#[test]
fn proposal_asset_vote_state_is_monotonic_after_threshold() {
    let asset = Pubkey::new_unique();
    let mut proposal_asset = ProposalAsset::new(0, 2, 255, asset);

    proposal_asset.increment_vote_count().unwrap();
    proposal_asset.add_use_vote_weight(10);

    assert_eq!(proposal_asset.vote_count, 1);
    assert_eq!(proposal_asset.use_vote_weight, 10);

    proposal_asset
        .set_threshold_state(ProposalAssetThresholdState::UseThresholdReached)
        .unwrap();

    assert_anchor_error(
        proposal_asset.set_threshold_state(ProposalAssetThresholdState::NotUseThresholdReached),
        "StateAlreadyFinalized",
        6047,
        "State already finalized",
    );
}

#[test]
fn proposal_asset_rejects_noop_threshold_transition() {
    let asset = Pubkey::new_unique();
    let mut proposal_asset = ProposalAsset::new(0, 0, 255, asset);

    assert_anchor_error(
        proposal_asset.set_threshold_state(ProposalAssetThresholdState::NoThresholdReached),
        "InvalidStateTransition",
        6048,
        "Invalid state transition",
    );
}

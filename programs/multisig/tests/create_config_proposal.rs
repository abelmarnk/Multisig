#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::CreateConfigProposalInstructionArgs, ConfigChange, ConfigType, Permissions,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, set_group_member_permissions, setup_asset_mint, setup_group, threshold,
};

// Create config proposal validates asset targets, permissons, deadlines, and config types.
struct TestSetup {}

enum Scenario {
    Default,
    MissingAsset,
    NoProposePermission,
    ExpiredDeadline,
    UseConfigOnGroup,
    OverlappingThreshold,
    MismatchedAssetInConfig,
}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        scenario: Scenario,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        let (config_change, asset, deadline) = match scenario {
            Scenario::Default => (
                ConfigChange::ChangeGroupConfig {
                    config_type: ConfigType::MinimumVoteCount(1),
                },
                None,
                1000i64,
            ),
            Scenario::MissingAsset => (
                ConfigChange::AddAssetMember {
                    member: group_setup.member_keys[1],
                    weight: 1,
                    permissions: Permissions::from_flags(true, true),
                    asset_address: asset_setup.asset_address,
                },
                None, // intentionally omitted despite being an asset change
                1000i64,
            ),
            Scenario::NoProposePermission => {
                set_group_member_permissions(
                    svm,
                    group_setup.group,
                    group_setup.payer.pubkey(),
                    Permissions::from_flags(false, false),
                )?;
                (
                    ConfigChange::ChangeGroupConfig {
                        config_type: ConfigType::MinimumVoteCount(1),
                    },
                    None,
                    1000i64,
                )
            }
            Scenario::ExpiredDeadline => (
                ConfigChange::ChangeGroupConfig {
                    config_type: ConfigType::MinimumVoteCount(1),
                },
                None,
                -1i64, // LiteSVM clock starts at 0 -> already expired
            ),
            Scenario::UseConfigOnGroup => (
                // Use/NotUse are only valid for asset proposals; using them on a group change
                // triggers UnexpectedConfigChange inside validate_group_config_type.
                ConfigChange::ChangeGroupConfig {
                    config_type: ConfigType::Use(threshold(1, 2)),
                },
                None,
                1000i64,
            ),
            Scenario::OverlappingThreshold => (
                // The group has not_change_config_threshold = 2/3.
                // A new change_config threshold of 1/3 overlaps because 1/3 + 2/3 == 1 ≤ 1.
                ConfigChange::ChangeGroupConfig {
                    config_type: ConfigType::ChangeConfig(threshold(1, 3)),
                },
                None,
                1000i64,
            ),
            Scenario::MismatchedAssetInConfig => {
                let other_asset = setup_asset_mint(svm, &group_setup)?;
                (
                    ConfigChange::AddAssetMember {
                        member: group_setup.member_keys[1],
                        weight: 1,
                        permissions: Permissions::from_flags(true, true),
                        // Config names a different asset than the one we'll pass in the account
                        asset_address: other_asset.asset_address,
                    },
                    Some(asset_setup.asset_address),
                    1000i64,
                )
            }
        };

        let args = CreateConfigProposalInstructionArgs {
            proposal_seed: solana_sdk::pubkey::Pubkey::new_unique(),
            timelock_offset: 0,
            proposal_deadline_timestamp: deadline,
            config_change,
        };

        let create_config_proposal =
            sdk::create_config_proposal(args, group_setup.group, group_setup.payer.pubkey(), asset);

        Ok(([create_config_proposal], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Default)
    }

    pub fn with_missing_asset(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::MissingAsset)
    }

    pub fn with_no_propose_permission(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::NoProposePermission)
    }

    pub fn with_expired_deadline(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::ExpiredDeadline)
    }

    pub fn with_use_config_on_group(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::UseConfigOnGroup)
    }

    pub fn with_overlapping_threshold(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::OverlappingThreshold)
    }

    pub fn with_mismatched_asset_in_config(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::MismatchedAssetInConfig)
    }
}

#[test]
fn test_create_config_proposal_success() {
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
fn test_create_config_proposal_fails_with_missing_asset() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_missing_asset(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::AssetNotProvided);
}

#[test]
fn test_create_config_proposal_fails_without_propose_permission() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_no_propose_permission(&mut svm);
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
        multisig::MultisigError::InsufficientPermissions,
    );
}

#[test]
fn test_create_config_proposal_fails_with_expired_deadline() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_expired_deadline(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::ProposalExpired);
}

#[test]
fn test_create_config_proposal_fails_with_use_config_on_group() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_use_config_on_group(&mut svm);
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
        multisig::MultisigError::UnexpectedConfigChange,
    );
}

#[test]
fn test_create_config_proposal_fails_with_overlapping_threshold() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_overlapping_threshold(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::InvalidThreshold);
}

#[test]
fn test_create_config_proposal_fails_with_mismatched_asset_in_config() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_mismatched_asset_in_config(&mut svm);
    let (instructions, signers) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, multisig::MultisigError::InvalidAsset);
}

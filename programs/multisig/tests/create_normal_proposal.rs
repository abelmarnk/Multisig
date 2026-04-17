#![cfg(feature = "test-helpers")]
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{
    instructions::CreateNormalProposalInstructionArgs, AssetIndex, Permissions,
    SerializableInstruction,
};
use multisig_sdk as sdk;
use solana_sdk::{instruction::Instruction, signer::Signer, transaction::Transaction};

mod common;
use common::{
    add_multisig_program, create_token_account_at, set_group_member_permissions, setup_asset_mint,
    setup_group, to_serializable,
};

// Create normal proposal should reject empty assets and accept a valid instruction hash.
struct TestSetup {}

enum Scenario {
    Default,
    EmptyAssets,
    DuplicateAssetIndex,
    MissingGroupProposePermission,
}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        scenario: Scenario,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;

        if matches!(scenario, Scenario::MissingGroupProposePermission) {
            // Strip the propose bit from the payer's group membership AFTER asset setup
            // (asset setup still needs the adder to have add_asset permission).
            set_group_member_permissions(
                svm,
                group_setup.group,
                group_setup.payer.pubkey(),
                Permissions::from_flags(false, false),
            )?;
        }

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

        let (asset_keys, asset_indices, authority_bumps) = match scenario {
            Scenario::EmptyAssets => (Vec::new(), Vec::new(), Vec::new()),
            Scenario::DuplicateAssetIndex => {
                let other_asset = setup_asset_mint(svm, &group_setup)?;
                let mut assets = [
                    (
                        asset_setup.mint,
                        sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump,
                    ),
                    (
                        other_asset.mint,
                        sdk::asset_authority_pda(&group_setup.group, &other_asset.mint).bump,
                    ),
                ];
                assets.sort_by_key(|(asset, _)| *asset);
                (
                    assets.iter().map(|(asset, _)| *asset).collect(),
                    vec![
                        AssetIndex {
                            instruction_index: 0,
                            account_index: 0,
                        },
                        AssetIndex {
                            instruction_index: 0,
                            account_index: 0,
                        },
                    ],
                    assets.iter().map(|(_, bump)| *bump).collect(),
                )
            }
            Scenario::Default | Scenario::MissingGroupProposePermission => (
                vec![asset_setup.mint],
                vec![AssetIndex {
                    instruction_index: 0,
                    account_index: 0,
                }],
                vec![sdk::asset_authority_pda(&group_setup.group, &asset_setup.mint).bump],
            ),
        };

        let args = CreateNormalProposalInstructionArgs {
            proposal_seed: solana_sdk::pubkey::Pubkey::new_unique(),
            asset_keys,
            asset_indices,
            authority_bumps,
            timelock_offset: 0,
            proposal_deadline_timestamp: 1000,
            instruction_hashes,
        };

        let create_normal_proposal =
            sdk::create_normal_proposal(args, group_setup.group, group_setup.payer.pubkey());

        Ok(([create_normal_proposal], vec![group_setup.payer]))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::Default)
    }

    pub fn with_empty_assets(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::EmptyAssets)
    }

    pub fn with_duplicate_asset_index(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::DuplicateAssetIndex)
    }

    pub fn with_missing_group_propose_permission(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<solana_sdk::signature::Keypair>)> {
        Self::builder(svm, Scenario::MissingGroupProposePermission)
    }
}

#[test]
fn test_create_normal_proposal_success() {
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
fn test_create_normal_proposal_fails_with_empty_assets() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_empty_assets(&mut svm);
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
fn test_create_normal_proposal_fails_with_duplicate_asset_index() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_duplicate_asset_index(&mut svm);
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
        multisig::MultisigError::InvalidAssetIndex,
    );
}

#[test]
fn test_create_normal_proposal_fails_without_propose_permission() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_missing_group_propose_permission(&mut svm);
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

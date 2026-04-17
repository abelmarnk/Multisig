#![cfg(feature = "test-helpers")]
use anchor_spl::token_interface::spl_token_2022;
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{instructions::AddAssetMintInstructionArgs, Permissions};
use multisig_sdk as sdk;
use solana_sdk::{
    instruction::Instruction, program_option::COption, signature::Keypair, signer::Signer,
    transaction::Transaction,
};

mod common;
use common::{
    add_multisig_program, create_mint_with_keypair, create_token_2022_mint_with_transfer_fee,
    get_asset_authority, set_group_member_permissions, setup_group, threshold,
};

// The add asset mint instruction requires an add-asset permissioned member and a mint whose
// authorities match the derived asset authority.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        use_wrong_authority: bool,
        token_2022_transfer_fee: bool,
    ) -> Result<([Instruction; 1], Vec<Keypair>)> {
        let group_setup = setup_group(svm)?;
        let payer = group_setup.payer;
        let group = group_setup.group;
        let member_keys = group_setup.member_keys;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &mint);
        let authority = if use_wrong_authority {
            Keypair::new().pubkey()
        } else {
            asset_authority
        };

        let token_program = if token_2022_transfer_fee {
            create_token_2022_mint_with_transfer_fee(svm, &mint_keypair, &authority, &authority)?;
            spl_token_2022::ID
        } else {
            create_mint_with_keypair(
                svm,
                &mint_keypair,
                COption::Some(&authority),
                COption::Some(&authority),
                true,
            )?;
            spl_token::ID
        };

        let args = AddAssetMintInstructionArgs {
            member_key_1: member_keys[0],
            member_key_2: member_keys[1],
            member_key_3: member_keys[2],
            initial_weights: [1, 1, 1],
            initial_permissions: [Permissions::from_flags(true, true); 3],
            use_threshold: threshold(1, 2),
            not_use_threshold: threshold(2, 3),
            add_threshold: threshold(1, 2),
            not_add_threshold: threshold(2, 3),
            remove_threshold: threshold(1, 2),
            not_remove_threshold: threshold(2, 3),
            change_config_threshold: threshold(1, 2),
            not_change_config_threshold: threshold(2, 3),
            minimum_member_count: 2,
            minimum_vote_count: 2,
        };

        let add_asset_mint = sdk::add_asset_mint(args, payer.pubkey(), group, mint, token_program);

        Ok(([add_asset_mint], vec![payer]))
    }

    pub fn with_default(svm: &mut LiteSVM) -> Result<([Instruction; 1], Vec<Keypair>)> {
        Self::builder(svm, false, false)
    }

    pub fn with_wrong_authority(svm: &mut LiteSVM) -> Result<([Instruction; 1], Vec<Keypair>)> {
        Self::builder(svm, true, false)
    }

    pub fn with_token_2022_transfer_fee(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<Keypair>)> {
        Self::builder(svm, false, true)
    }

    /// Payer's GroupMember has no add_asset permission -> InsufficientPermissions
    pub fn with_no_add_asset_permission(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<Keypair>)> {
        let group_setup = setup_group(svm)?;
        let payer = group_setup.payer;
        let group = group_setup.group;
        let member_keys = group_setup.member_keys;

        // Revoke add_asset permission from the payer's group member
        set_group_member_permissions(
            svm,
            group,
            payer.pubkey(),
            Permissions::from_flags(false, false),
        )?;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &mint);

        create_mint_with_keypair(
            svm,
            &mint_keypair,
            COption::Some(&asset_authority),
            COption::Some(&asset_authority),
            true,
        )?;

        let args = AddAssetMintInstructionArgs {
            member_key_1: member_keys[0],
            member_key_2: member_keys[1],
            member_key_3: member_keys[2],
            initial_weights: [1, 1, 1],
            initial_permissions: [Permissions::from_flags(true, true); 3],
            use_threshold: threshold(1, 2),
            not_use_threshold: threshold(2, 3),
            add_threshold: threshold(1, 2),
            not_add_threshold: threshold(2, 3),
            remove_threshold: threshold(1, 2),
            not_remove_threshold: threshold(2, 3),
            change_config_threshold: threshold(1, 2),
            not_change_config_threshold: threshold(2, 3),
            minimum_member_count: 2,
            minimum_vote_count: 2,
        };

        let add_asset_mint = sdk::add_asset_mint(args, payer.pubkey(), group, mint, spl_token::ID);
        Ok(([add_asset_mint], vec![payer]))
    }

    /// Mint has a freeze authority that is not the asset authority -> InvalidMintFreezeAuthority
    pub fn with_wrong_freeze_authority(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<Keypair>)> {
        let group_setup = setup_group(svm)?;
        let payer = group_setup.payer;
        let group = group_setup.group;
        let member_keys = group_setup.member_keys;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &mint);
        let wrong_freeze = Keypair::new().pubkey(); // ≠ asset_authority

        create_mint_with_keypair(
            svm,
            &mint_keypair,
            COption::Some(&asset_authority), // correct mint authority
            COption::Some(&wrong_freeze),    // wrong freeze authority
            true,
        )?;

        let args = AddAssetMintInstructionArgs {
            member_key_1: member_keys[0],
            member_key_2: member_keys[1],
            member_key_3: member_keys[2],
            initial_weights: [1, 1, 1],
            initial_permissions: [Permissions::from_flags(true, true); 3],
            use_threshold: threshold(1, 2),
            not_use_threshold: threshold(2, 3),
            add_threshold: threshold(1, 2),
            not_add_threshold: threshold(2, 3),
            remove_threshold: threshold(1, 2),
            not_remove_threshold: threshold(2, 3),
            change_config_threshold: threshold(1, 2),
            not_change_config_threshold: threshold(2, 3),
            minimum_member_count: 2,
            minimum_vote_count: 2,
        };

        let add_asset_mint = sdk::add_asset_mint(args, payer.pubkey(), group, mint, spl_token::ID);
        Ok(([add_asset_mint], vec![payer]))
    }

    /// An initial weight exceeds the group's max_member_weight (100) -> InvalidMemberWeight
    pub fn with_weight_exceeds_max(svm: &mut LiteSVM) -> Result<([Instruction; 1], Vec<Keypair>)> {
        let group_setup = setup_group(svm)?;
        let payer = group_setup.payer;
        let group = group_setup.group;
        let member_keys = group_setup.member_keys;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &mint);

        create_mint_with_keypair(
            svm,
            &mint_keypair,
            COption::Some(&asset_authority),
            COption::Some(&asset_authority),
            true,
        )?;

        let args = AddAssetMintInstructionArgs {
            member_key_1: member_keys[0],
            member_key_2: member_keys[1],
            member_key_3: member_keys[2],
            initial_weights: [101, 101, 101], // 101 > max_member_weight=100
            initial_permissions: [Permissions::from_flags(true, true); 3],
            use_threshold: threshold(1, 2),
            not_use_threshold: threshold(2, 3),
            add_threshold: threshold(1, 2),
            not_add_threshold: threshold(2, 3),
            remove_threshold: threshold(1, 2),
            not_remove_threshold: threshold(2, 3),
            change_config_threshold: threshold(1, 2),
            not_change_config_threshold: threshold(2, 3),
            minimum_member_count: 2,
            minimum_vote_count: 2,
        };

        let add_asset_mint = sdk::add_asset_mint(args, payer.pubkey(), group, mint, spl_token::ID);
        Ok(([add_asset_mint], vec![payer]))
    }
}

#[test]
fn test_add_asset_mint_success() {
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
fn test_add_asset_mint_fails_with_wrong_authority() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_authority(&mut svm);
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
        multisig::MultisigError::InvalidMintMintAuthority,
    );
}

#[test]
fn test_add_asset_mint_rejects_token_2022_transfer_fee_extension() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_token_2022_transfer_fee(&mut svm);
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
        multisig::MultisigError::UnsupportedTokenExtensions,
    );
}

#[test]
fn test_add_asset_mint_fails_without_add_asset_permission() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_no_add_asset_permission(&mut svm);
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
fn test_add_asset_mint_fails_with_wrong_freeze_authority() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_freeze_authority(&mut svm);
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
        multisig::MultisigError::InvalidMintFreezeAuthority,
    );
}

#[test]
fn test_add_asset_mint_fails_when_weight_exceeds_max() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_weight_exceeds_max(&mut svm);
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
        multisig::MultisigError::InvalidMemberWeight,
    );
}

#![cfg(feature = "test-helpers")]
use anchor_spl::token_interface::spl_token_2022;
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{instructions::AddAssetTokenInstructionArgs, Permissions};
use multisig_sdk as sdk;
use solana_sdk::{
    instruction::Instruction, program_option::COption, signature::Keypair, signer::Signer,
    transaction::Transaction,
};

mod common;
use common::{
    add_multisig_program, create_mint_with_keypair,
    create_token_2022_account_with_transfer_fee_amount, create_token_account_at,
    get_asset_authority, set_group_member_permissions, setup_group, threshold,
};

// The add asset token instruction requires an initialized token account owned by the derived
// asset authority and without any delegate.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        use_wrong_owner: bool,
        token_2022_transfer_fee_amount: bool,
    ) -> Result<([Instruction; 1], Vec<Keypair>)> {
        let group_setup = setup_group(svm)?;
        let payer = group_setup.payer;
        let group = group_setup.group;
        let member_keys = group_setup.member_keys;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let token_keypair = Keypair::new();
        let token_account = token_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &token_account);
        let owner = if use_wrong_owner {
            Keypair::new().pubkey()
        } else {
            asset_authority
        };

        create_mint_with_keypair(
            svm,
            &mint_keypair,
            COption::Some(&asset_authority),
            COption::Some(&asset_authority),
            true,
        )?;

        let token_program = if token_2022_transfer_fee_amount {
            create_token_2022_account_with_transfer_fee_amount(svm, &token_account, &mint, &owner)?;
            spl_token_2022::ID
        } else {
            create_token_account_at(
                svm,
                &token_account,
                &mint,
                &owner,
                COption::None,
                spl_token::state::AccountState::Initialized,
                COption::None,
            )?;
            spl_token::ID
        };

        let args = AddAssetTokenInstructionArgs {
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

        let add_asset_token =
            sdk::add_asset_token(args, payer.pubkey(), group, token_account, token_program);

        Ok(([add_asset_token], vec![payer]))
    }

    pub fn with_default(svm: &mut LiteSVM) -> Result<([Instruction; 1], Vec<Keypair>)> {
        Self::builder(svm, false, false)
    }

    pub fn with_wrong_owner(svm: &mut LiteSVM) -> Result<([Instruction; 1], Vec<Keypair>)> {
        Self::builder(svm, true, false)
    }

    pub fn with_token_2022_transfer_fee_amount(
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

        set_group_member_permissions(
            svm,
            group,
            payer.pubkey(),
            Permissions::from_flags(false, false),
        )?;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let token_keypair = Keypair::new();
        let token_account = token_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &token_account);

        create_mint_with_keypair(
            svm,
            &mint_keypair,
            COption::Some(&asset_authority),
            COption::Some(&asset_authority),
            true,
        )?;
        create_token_account_at(
            svm,
            &token_account,
            &mint,
            &asset_authority,
            COption::None,
            spl_token::state::AccountState::Initialized,
            COption::None,
        )?;

        let args = AddAssetTokenInstructionArgs {
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

        let add_asset_token =
            sdk::add_asset_token(args, payer.pubkey(), group, token_account, spl_token::ID);
        Ok(([add_asset_token], vec![payer]))
    }

    /// Token account has a delegate set -> InvalidTokenDelegate
    pub fn with_delegate(svm: &mut LiteSVM) -> Result<([Instruction; 1], Vec<Keypair>)> {
        let group_setup = setup_group(svm)?;
        let payer = group_setup.payer;
        let group = group_setup.group;
        let member_keys = group_setup.member_keys;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let token_keypair = Keypair::new();
        let token_account = token_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &token_account);
        let delegate_key = Keypair::new().pubkey();

        create_mint_with_keypair(
            svm,
            &mint_keypair,
            COption::Some(&asset_authority),
            COption::Some(&asset_authority),
            true,
        )?;
        create_token_account_at(
            svm,
            &token_account,
            &mint,
            &asset_authority,
            COption::Some(&delegate_key), // delegate is set
            spl_token::state::AccountState::Initialized,
            COption::None,
        )?;

        let args = AddAssetTokenInstructionArgs {
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

        let add_asset_token =
            sdk::add_asset_token(args, payer.pubkey(), group, token_account, spl_token::ID);
        Ok(([add_asset_token], vec![payer]))
    }

    /// Token account has a close authority that is not the asset authority -> InvalidCloseAuthority
    pub fn with_wrong_close_authority(
        svm: &mut LiteSVM,
    ) -> Result<([Instruction; 1], Vec<Keypair>)> {
        let group_setup = setup_group(svm)?;
        let payer = group_setup.payer;
        let group = group_setup.group;
        let member_keys = group_setup.member_keys;

        let mint_keypair = Keypair::new();
        let mint = mint_keypair.pubkey();
        let token_keypair = Keypair::new();
        let token_account = token_keypair.pubkey();
        let asset_authority = get_asset_authority(&group, &token_account);
        let wrong_close_auth = Keypair::new().pubkey(); // ≠ asset_authority

        create_mint_with_keypair(
            svm,
            &mint_keypair,
            COption::Some(&asset_authority),
            COption::Some(&asset_authority),
            true,
        )?;
        create_token_account_at(
            svm,
            &token_account,
            &mint,
            &asset_authority,
            COption::None,
            spl_token::state::AccountState::Initialized,
            COption::Some(&wrong_close_auth), // wrong close authority
        )?;

        let args = AddAssetTokenInstructionArgs {
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

        let add_asset_token =
            sdk::add_asset_token(args, payer.pubkey(), group, token_account, spl_token::ID);
        Ok(([add_asset_token], vec![payer]))
    }
}

#[test]
fn test_add_asset_token_success() {
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
fn test_add_asset_token_fails_with_wrong_owner() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_owner(&mut svm);
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
        multisig::MultisigError::InvalidTokenOwner,
    );
}

#[test]
fn test_add_asset_token_rejects_token_2022_transfer_fee_amount_extension() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_token_2022_transfer_fee_amount(&mut svm);
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
fn test_add_asset_token_fails_without_add_asset_permission() {
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
fn test_add_asset_token_fails_with_delegate() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_delegate(&mut svm);
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
        multisig::MultisigError::InvalidTokenDelegate,
    );
}

#[test]
fn test_add_asset_token_fails_with_wrong_close_authority() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_close_authority(&mut svm);
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
        multisig::MultisigError::InvalidCloseAuthority,
    );
}

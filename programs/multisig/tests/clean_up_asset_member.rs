#![cfg(feature = "test-helpers")]
use anchor_lang::AccountDeserialize;
use anyhow::Result;
use litesvm::LiteSVM;
use multisig::{Asset, MultisigError};
use solana_sdk::{
    account::Account, instruction::Instruction, signer::Signer, system_program,
    transaction::Transaction,
};

mod common;
use common::{add_multisig_program, get_group_member, setup_asset_mint, setup_group};

// Clean up asset member requires the group member account to be closed.
struct TestSetup {}

impl TestSetup {
    fn builder(
        svm: &mut LiteSVM,
        close_group_member: bool,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let member = group_setup.member_keys[0];
        let group_member = get_group_member(&group_setup.group, &member);

        if close_group_member {
            let closed = Account {
                lamports: 0,
                data: Vec::new(),
                owner: system_program::ID,
                executable: false,
                rent_epoch: 0,
            };
            svm.set_account(group_member, closed)?;
        }

        let clean_up = multisig_sdk::clean_up_asset_member(
            group_setup.group,
            asset_setup.asset_address,
            member,
            group_setup.payer.pubkey(),
        );

        Ok(([clean_up], vec![group_setup.payer], asset_setup.asset))
    }

    pub fn with_default(
        svm: &mut LiteSVM,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        Self::builder(svm, true)
    }

    pub fn with_active_group_member(
        svm: &mut LiteSVM,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        Self::builder(svm, false)
    }

    /// Wrong rent collector - does not match group.rent_collector.
    pub fn with_wrong_rent_collector(
        svm: &mut LiteSVM,
    ) -> Result<(
        [Instruction; 1],
        Vec<solana_sdk::signature::Keypair>,
        solana_sdk::pubkey::Pubkey,
    )> {
        let group_setup = setup_group(svm)?;
        let asset_setup = setup_asset_mint(svm, &group_setup)?;
        let member = group_setup.member_keys[0];
        let group_member = get_group_member(&group_setup.group, &member);

        // Close the group member so we get past the GroupMemberStillActive check.
        svm.set_account(
            group_member,
            Account {
                lamports: 0,
                data: Vec::new(),
                owner: system_program::ID,
                executable: false,
                rent_epoch: 0,
            },
        )?;

        let wrong_rent_collector = solana_sdk::signature::Keypair::new().pubkey();
        let clean_up = multisig_sdk::clean_up_asset_member(
            group_setup.group,
            asset_setup.asset_address,
            member,
            wrong_rent_collector,
        );
        Ok(([clean_up], vec![group_setup.payer], asset_setup.asset))
    }
}

#[test]
fn test_clean_up_asset_member_success() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_default(&mut svm);
    let (instructions, signers, asset) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_transaction_success(result);
    let account = svm.get_account(&asset).expect("asset account");
    let mut data = account.data.as_slice();
    let asset_data = Asset::try_deserialize(&mut data).expect("asset deserialize");
    assert_eq!(asset_data.member_count, 2);
}

#[test]
fn test_clean_up_asset_member_fails_when_group_member_active() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_active_group_member(&mut svm);
    let (instructions, signers, _) = match result {
        Ok(result) => result,
        Err(error) => panic!("Failed to create instruction: {}", error),
    };

    let payer = signers[0].pubkey();
    let recent_blockhash = svm.latest_blockhash();
    let transaction =
        Transaction::new_signed_with_payer(&instructions, Some(&payer), &signers, recent_blockhash);

    let result = svm.send_transaction(transaction);
    common::assert_multisig_instruction_error(result, 0, MultisigError::GroupMemberStillActive);
}

#[test]
fn test_clean_up_asset_member_fails_with_wrong_rent_collector() {
    let mut svm = LiteSVM::new();
    add_multisig_program(&mut svm).expect("program load");

    let result = TestSetup::with_wrong_rent_collector(&mut svm);
    let (instructions, signers, _) = match result {
        Ok(r) => r,
        Err(e) => panic!("setup failed: {e}"),
    };

    let payer = signers[0].pubkey();
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer),
        &signers,
        svm.latest_blockhash(),
    );
    common::assert_multisig_instruction_error(
        svm.send_transaction(tx),
        0,
        MultisigError::UnexpectedRentCollector,
    );
}

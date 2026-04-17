#![allow(dead_code)] // Not sure why there are unused function errors, would look into it later.

use std::path::Path;

use anchor_lang::{AccountDeserialize, AccountSerialize, Space};
use anchor_spl::token_interface::spl_token_2022::{
    self,
    extension::{
        transfer_fee::{TransferFeeAmount, TransferFeeConfig},
        AccountType, ExtensionType,
    },
    state::{
        Account as Token2022Account, AccountState as Token2022AccountState, Mint as Token2022Mint,
    },
};
use anyhow::{Context, Result};
#[cfg(feature = "test-helpers")]
use litesvm::LiteSVM;
use multisig::{
    instructions::{
        AddAssetMintInstructionArgs, AddAssetTokenInstructionArgs, CreateGroupInstructionArgs,
    },
    proposal::{
        ConfigProposal, EmergencyResetProposal, NormalProposal, ProposalState, ProposalTransaction,
    },
    ConfigChange, FractionalThreshold, Group, GroupMember, MultisigError, Permissions,
    SerailizableAccountMeta, SerializableInstruction, ID as MULTISIG_PROGRAM_ID,
};
use multisig_sdk as sdk;
use rand::Rng;
use solana_sdk::{
    account::Account,
    instruction::{Instruction, InstructionError},
    program_option::COption,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    transaction::TransactionError,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    solana_program::program_pack::Pack,
    state::{Account as TokenAccount, AccountState, Mint},
    ID as TOKEN_PROGRAM_ID,
};

use litesvm::types::TransactionResult;

pub fn option_to_c_option<T>(option: Option<T>) -> COption<T> {
    option.map_or(COption::None, |value| COption::Some(value))
}

pub fn get_group(seed: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"group", seed.as_ref()], &MULTISIG_PROGRAM_ID).0
}

pub fn get_group_member(group: &Pubkey, member: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"member", group.as_ref(), member.as_ref()],
        &MULTISIG_PROGRAM_ID,
    )
    .0
}

pub fn get_asset(group: &Pubkey, asset_address: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"asset", group.as_ref(), asset_address.as_ref()],
        &MULTISIG_PROGRAM_ID,
    )
    .0
}

pub fn get_asset_authority(group: &Pubkey, asset_address: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"authority", group.as_ref(), asset_address.as_ref()],
        &MULTISIG_PROGRAM_ID,
    )
    .0
}

pub fn get_asset_member(group: &Pubkey, asset_address: &Pubkey, member: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"asset-member",
            group.as_ref(),
            asset_address.as_ref(),
            member.as_ref(),
        ],
        &MULTISIG_PROGRAM_ID,
    )
    .0
}

pub fn get_proposal(group: &Pubkey, proposal_seed: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"proposal", group.as_ref(), proposal_seed.as_ref()],
        &MULTISIG_PROGRAM_ID,
    )
    .0
}

pub fn get_proposal_transaction(proposal: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"proposal-transaction", proposal.as_ref()],
        &MULTISIG_PROGRAM_ID,
    )
    .0
}

pub fn get_vote_record(
    group: &Pubkey,
    proposal: &Pubkey,
    voter: &Pubkey,
    asset_index: Option<u8>,
) -> Pubkey {
    match asset_index {
        Some(asset_index) => {
            Pubkey::find_program_address(
                &[
                    b"vote-record",
                    group.as_ref(),
                    proposal.as_ref(),
                    voter.as_ref(),
                    &[asset_index],
                ],
                &MULTISIG_PROGRAM_ID,
            )
            .0
        }
        None => {
            Pubkey::find_program_address(
                &[
                    b"vote-record",
                    group.as_ref(),
                    proposal.as_ref(),
                    voter.as_ref(),
                ],
                &MULTISIG_PROGRAM_ID,
            )
            .0
        }
    }
}

pub fn add_multisig_program(svm: &mut LiteSVM) -> Result<()> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/multisig.so");

    svm.add_program_from_file(MULTISIG_PROGRAM_ID, &path)?;

    Ok(())
}

pub fn threshold(numerator: u32, denominator: u32) -> FractionalThreshold {
    FractionalThreshold::new_from_values(numerator, denominator).unwrap()
}

pub fn permissions() -> Permissions {
    Permissions::from_flags(true, true)
}

pub fn get_invalid_threshold(rng: &mut rand::rngs::ThreadRng) -> FractionalThreshold {
    let value = rng.random::<u8>() % 4;

    match value {
        0 => FractionalThreshold::from_unchecked(1, 0),
        1 => FractionalThreshold::from_unchecked(0, 1),
        2 => FractionalThreshold::from_unchecked(2, 1),
        _ => {
            let numerator = rng.random::<u32>();

            let mut denominator = rng.random::<u32>();

            while denominator.lt(&numerator) {
                denominator = rng.random::<u32>();
            }

            FractionalThreshold::from_unchecked(numerator, denominator)
        }
    }
}

pub fn get_invalid_permissions(rng: &mut rand::rngs::ThreadRng) -> Permissions {
    let value = rng.random::<u8>() % 4;

    Permissions::from_unchecked(match value {
        0 => 0b00000011u8 | 0b00000100u8,
        1 => 0b00000011u8 | 0b00001000u8,
        2 => 0b00000011u8 | 0b00010000u8,
        _ => 0b10000000u8 | rng.random::<u8>(),
    })
}

pub fn send_tx(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instructions: Vec<solana_sdk::instruction::Instruction>,
    extra_signers: &[&Keypair],
) -> Result<()> {
    let mut signers: Vec<&Keypair> = Vec::with_capacity(1 + extra_signers.len());
    signers.push(payer);
    signers.extend_from_slice(extra_signers);

    let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .map(|_| ())
        .map_err(|err| anyhow::anyhow!("transaction failed: {err:?}"))?;

    Ok(())
}

#[track_caller]
pub fn assert_instruction_error(
    result: TransactionResult,
    instruction_index: u8,
    expected_error_code: u32,
) {
    let expected = TransactionError::InstructionError(
        instruction_index,
        InstructionError::Custom(expected_error_code),
    );

    match result {
        Ok(meta) => panic!(
            "Expected transaction to fail with {expected:?}, but it succeeded (compute units: {:?})",
            meta.compute_units_consumed
        ),
        Err(error) => assert_eq!(error.err, expected),
    }
}

#[track_caller]
pub fn assert_multisig_instruction_error(
    result: TransactionResult,
    instruction_index: u8,
    error: MultisigError,
) {
    assert_instruction_error(result, instruction_index, u32::from(error));
}

#[track_caller]
pub fn assert_transaction_success(result: TransactionResult) {
    match result {
        Ok(_) => {}
        Err(error) => panic!("Expected transaction to succeed, but it failed: {error:?}"),
    }
}

pub struct GroupSetup {
    pub payer: Keypair,
    pub members: [Keypair; 4],
    pub member_keys: [Pubkey; 5],
    pub group_seed: Pubkey,
    pub group: Pubkey,
}

pub fn setup_group(svm: &mut LiteSVM) -> Result<GroupSetup> {
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 1_000_000_000)
        .expect("Could not airdrop to payer");

    let members: [Keypair; 4] = std::array::from_fn(|_| Keypair::new());
    for member in members.iter() {
        svm.airdrop(&member.pubkey(), 1_000_000_000)
            .expect("Could not airdrop to member");
    }

    let member_keys = [
        payer.pubkey(),
        members[0].pubkey(),
        members[1].pubkey(),
        members[2].pubkey(),
        members[3].pubkey(),
    ];

    let group_seed = Pubkey::new_unique();
    let group = get_group(&group_seed);

    let create_group_args = CreateGroupInstructionArgs {
        group_seed,
        rent_collector: payer.pubkey(),
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
        member_weights: [1; 5],
        member_permissions: [permissions(); 5],
    };

    let ix = sdk::create_group(create_group_args, payer.pubkey(), member_keys);
    send_tx(svm, &payer, vec![ix], &[])?;

    Ok(GroupSetup {
        payer,
        members,
        member_keys,
        group_seed,
        group,
    })
}

/// Create a mint into the svm with the keypair, owner and mint & freeze authority
pub fn create_mint(
    svm: &mut LiteSVM,
    maybe_mint_authority_key: COption<&Pubkey>,
    maybe_freeze_authority_key: COption<&Pubkey>,
    is_initialized: bool,
) -> Result<Keypair> {
    let mint_keypair = Keypair::new();

    create_mint_with_keypair(
        svm,
        &mint_keypair,
        maybe_mint_authority_key,
        maybe_freeze_authority_key,
        is_initialized,
    )?;

    Ok(mint_keypair)
}

pub fn create_mint_with_keypair(
    svm: &mut LiteSVM,
    mint_keypair: &Keypair,
    maybe_mint_authority_key: COption<&Pubkey>,
    maybe_freeze_authority_key: COption<&Pubkey>,
    is_initialized: bool,
) -> Result<()> {
    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);

    let mint = Mint {
        mint_authority: maybe_mint_authority_key.cloned(),
        supply: 100_000_000_000,
        decimals: 9,
        is_initialized,
        freeze_authority: maybe_freeze_authority_key.cloned(),
    };

    let mut mint_data = vec![0u8; Mint::LEN];

    Mint::pack(mint, &mut mint_data).unwrap();

    let mint_account = Account {
        lamports: rent,
        data: mint_data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    svm.set_account(mint_keypair.pubkey(), mint_account)?;

    Ok(())
}

/// Create a token account into the svm with the mint, owner, state and authorities
pub fn create_token_account(
    svm: &mut LiteSVM,
    mint: &Pubkey,
    owner: &Pubkey,
    delegate: COption<&Pubkey>,
    state: AccountState,
    close_authority: COption<&Pubkey>,
) -> Result<Pubkey> {
    let token_account_key = get_associated_token_address(owner, mint);

    let rent = svm.minimum_balance_for_rent_exemption(TokenAccount::LEN);

    let token_account = TokenAccount {
        mint: *mint,
        owner: *owner,
        amount: 1_000_000_000_000,
        delegate: delegate.cloned(),
        state,
        is_native: COption::None,
        delegated_amount: delegate.map(|_| 50_000_000_000).unwrap_or(0),
        close_authority: close_authority.cloned(),
    };

    let mut token_account_data = vec![0u8; TokenAccount::LEN];

    TokenAccount::pack(token_account, &mut token_account_data).unwrap();

    let token_account_account = Account {
        lamports: rent,
        data: token_account_data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    svm.set_account(token_account_key, token_account_account)?;

    Ok(token_account_key)
}

/// Create a token account into the svm at a provided address.
pub fn create_token_account_at(
    svm: &mut LiteSVM,
    token_account_key: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    delegate: COption<&Pubkey>,
    state: AccountState,
    close_authority: COption<&Pubkey>,
) -> Result<Pubkey> {
    let rent = svm.minimum_balance_for_rent_exemption(TokenAccount::LEN);

    let token_account = TokenAccount {
        mint: *mint,
        owner: *owner,
        amount: 1_000_000_000_000,
        delegate: delegate.cloned(),
        state,
        is_native: COption::None,
        delegated_amount: delegate.map(|_| 50_000_000_000).unwrap_or(0),
        close_authority: close_authority.cloned(),
    };

    let mut token_account_data = vec![0u8; TokenAccount::LEN];

    TokenAccount::pack(token_account, &mut token_account_data).unwrap();

    let token_account_account = Account {
        lamports: rent,
        data: token_account_data,
        owner: TOKEN_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };

    svm.set_account(*token_account_key, token_account_account)?;

    Ok(*token_account_key)
}

pub fn create_token_2022_mint_with_transfer_fee(
    svm: &mut LiteSVM,
    mint_keypair: &Keypair,
    mint_authority: &Pubkey,
    freeze_authority: &Pubkey,
) -> Result<()> {
    let len = ExtensionType::try_calculate_account_len::<Token2022Mint>(&[
        ExtensionType::TransferFeeConfig,
    ])
    .context("calculate token-2022 mint extension length")?;
    let rent = svm.minimum_balance_for_rent_exemption(len);
    let mut data = vec![0u8; len];

    let mint = Token2022Mint {
        mint_authority: COption::Some(*mint_authority),
        supply: 100_000_000_000,
        decimals: 9,
        is_initialized: true,
        freeze_authority: COption::Some(*freeze_authority),
    };
    Token2022Mint::pack(mint, &mut data[..Token2022Mint::LEN])
        .context("pack token-2022 mint base")?;
    let mut offset = Token2022Account::LEN;
    data[offset] = AccountType::Mint.into();
    offset += 1;
    data[offset..offset + 2]
        .copy_from_slice(&(ExtensionType::TransferFeeConfig as u16).to_le_bytes());
    offset += 2;
    data[offset..offset + 2]
        .copy_from_slice(&(std::mem::size_of::<TransferFeeConfig>() as u16).to_le_bytes());

    let mint_account = Account {
        lamports: rent,
        data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    };

    svm.set_account(mint_keypair.pubkey(), mint_account)
        .context("set token-2022 mint account")?;
    Ok(())
}

pub fn create_token_2022_account_with_transfer_fee_amount(
    svm: &mut LiteSVM,
    token_account_key: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<()> {
    let len = ExtensionType::try_calculate_account_len::<Token2022Account>(&[
        ExtensionType::TransferFeeAmount,
    ])
    .context("calculate token-2022 account extension length")?;
    let rent = svm.minimum_balance_for_rent_exemption(len);
    let mut data = vec![0u8; len];

    let token = Token2022Account {
        mint: *mint,
        owner: *owner,
        amount: 1_000_000_000_000,
        delegate: COption::None,
        state: Token2022AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    Token2022Account::pack(token, &mut data[..Token2022Account::LEN])
        .context("pack token-2022 account base")?;
    let mut offset = Token2022Account::LEN;
    data[offset] = AccountType::Account.into();
    offset += 1;
    data[offset..offset + 2]
        .copy_from_slice(&(ExtensionType::TransferFeeAmount as u16).to_le_bytes());
    offset += 2;
    data[offset..offset + 2]
        .copy_from_slice(&(std::mem::size_of::<TransferFeeAmount>() as u16).to_le_bytes());

    let token_account = Account {
        lamports: rent,
        data,
        owner: spl_token_2022::ID,
        executable: false,
        rent_epoch: 0,
    };

    svm.set_account(*token_account_key, token_account)
        .context("set token-2022 token account")?;
    Ok(())
}

pub struct AssetSetup {
    pub asset_address: Pubkey,
    pub asset: Pubkey,
    pub asset_authority: Pubkey,
    pub mint: Pubkey,
}

pub fn setup_asset_mint(svm: &mut LiteSVM, group_setup: &GroupSetup) -> Result<AssetSetup> {
    setup_asset_mint_with_permissions(
        svm,
        group_setup,
        [permissions(), permissions(), permissions()],
    )
}

pub fn setup_asset_mint_with_permissions(
    svm: &mut LiteSVM,
    group_setup: &GroupSetup,
    initial_permissions: [Permissions; 3],
) -> Result<AssetSetup> {
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();
    let asset_authority = get_asset_authority(&group_setup.group, &mint);

    create_mint_with_keypair(
        svm,
        &mint_keypair,
        option_to_c_option(Some(&asset_authority)),
        option_to_c_option(Some(&asset_authority)),
        true,
    )?;

    let add_asset_mint_args = AddAssetMintInstructionArgs {
        member_key_1: group_setup.member_keys[0],
        member_key_2: group_setup.member_keys[1],
        member_key_3: group_setup.member_keys[2],
        initial_weights: [1, 1, 1],
        initial_permissions,
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

    let ix = sdk::add_asset_mint(
        add_asset_mint_args,
        group_setup.payer.pubkey(),
        group_setup.group,
        mint,
        spl_token::ID,
    );
    send_tx(svm, &group_setup.payer, vec![ix], &[])?;

    Ok(AssetSetup {
        asset_address: mint,
        asset: get_asset(&group_setup.group, &mint),
        asset_authority,
        mint,
    })
}

pub fn setup_asset_token(svm: &mut LiteSVM, group_setup: &GroupSetup) -> Result<AssetSetup> {
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();
    let token_keypair = Keypair::new();
    let token_account = token_keypair.pubkey();
    let asset_authority = get_asset_authority(&group_setup.group, &token_account);

    create_mint_with_keypair(
        svm,
        &mint_keypair,
        option_to_c_option(Some(&asset_authority)),
        option_to_c_option(Some(&asset_authority)),
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

    let add_asset_token_args = AddAssetTokenInstructionArgs {
        member_key_1: group_setup.member_keys[0],
        member_key_2: group_setup.member_keys[1],
        member_key_3: group_setup.member_keys[2],
        initial_weights: [1, 1, 1],
        initial_permissions: [permissions(), permissions(), permissions()],
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

    let ix = sdk::add_asset_token(
        add_asset_token_args,
        group_setup.payer.pubkey(),
        group_setup.group,
        token_account,
        spl_token::ID,
    );
    send_tx(svm, &group_setup.payer, vec![ix], &[])?;

    Ok(AssetSetup {
        asset_address: token_account,
        asset: get_asset(&group_setup.group, &token_account),
        asset_authority,
        mint,
    })
}

pub fn to_serializable(ix: &Instruction) -> SerializableInstruction {
    let accounts: Vec<SerailizableAccountMeta> = ix
        .accounts
        .iter()
        .map(|meta| SerailizableAccountMeta {
            key: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    SerializableInstruction {
        program_id: ix.program_id,
        accounts,
        data: ix.data.clone(),
    }
}

pub fn get_emergency_reset_proposal(group: &Pubkey, proposal_seed: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"emergency-reset", group.as_ref(), proposal_seed.as_ref()],
        &MULTISIG_PROGRAM_ID,
    )
    .0
}

pub fn read_emergency_reset_proposal(
    svm: &LiteSVM,
    proposal: &Pubkey,
) -> Result<EmergencyResetProposal> {
    use anchor_lang::AccountDeserialize;
    let account = svm
        .get_account(proposal)
        .ok_or_else(|| anyhow::anyhow!("emergency reset proposal account not found: {proposal}"))?;
    Ok(EmergencyResetProposal::try_deserialize(
        &mut account.data.as_ref(),
    )?)
}

pub fn insert_config_proposal(
    svm: &mut LiteSVM,
    proposal: Pubkey,
    data: ConfigProposal,
) -> Result<()> {
    let mut serialized = Vec::with_capacity(8 + ConfigProposal::INIT_SPACE);
    data.try_serialize(&mut serialized)?;
    let rent = svm.minimum_balance_for_rent_exemption(serialized.len());
    let account = Account {
        lamports: rent,
        data: serialized,
        owner: MULTISIG_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(proposal, account)?;
    Ok(())
}

pub fn insert_normal_proposal(
    svm: &mut LiteSVM,
    proposal: Pubkey,
    data: NormalProposal,
) -> Result<()> {
    let asset_len = data.assets.len();
    let hash_count = data.instruction_hashes.len();
    let mut serialized = Vec::with_capacity(8 + NormalProposal::get_size(asset_len, hash_count));
    data.try_serialize(&mut serialized)?;
    let rent = svm.minimum_balance_for_rent_exemption(serialized.len());
    let account = Account {
        lamports: rent,
        data: serialized,
        owner: MULTISIG_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(proposal, account)?;
    Ok(())
}

pub fn set_config_proposal_state(
    svm: &mut LiteSVM,
    proposal: Pubkey,
    state: ProposalState,
    passed_timestamp: Option<i64>,
) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = ConfigProposal::try_deserialize(&mut data)?;
    proposal_data.set_state(state)?;
    if let Some(timestamp) = passed_timestamp {
        proposal_data.set_proposal_passed_timestamp(timestamp);
    }
    let mut serialized = Vec::with_capacity(account.data.len());
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

/// Replace the `config_change` field of an existing config proposal.
/// Useful for injecting config types that are rejected at proposal-creation time.
pub fn set_config_proposal_config_change(
    svm: &mut LiteSVM,
    proposal: Pubkey,
    config_change: ConfigChange,
) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = ConfigProposal::try_deserialize(&mut data)?;
    proposal_data.config_change = config_change;
    let mut serialized = Vec::with_capacity(account.data.len() + 16);
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

pub fn set_normal_proposal_state(
    svm: &mut LiteSVM,
    proposal: Pubkey,
    state: ProposalState,
    passed_timestamp: Option<i64>,
) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = NormalProposal::try_deserialize(&mut data)?;
    proposal_data.set_state(state)?;
    if let Some(timestamp) = passed_timestamp {
        proposal_data.set_proposal_passed_timestamp(timestamp);
    }
    let mut serialized = Vec::with_capacity(account.data.len());
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

pub fn insert_proposal_transaction(
    svm: &mut LiteSVM,
    proposal_transaction: Pubkey,
    data: ProposalTransaction,
) -> Result<()> {
    let asset_len = data.asset_indices.len();
    let instructions_total_size = 4 + data
        .instructions
        .iter()
        .map(|ix| ix.get_size())
        .sum::<usize>();
    let mut serialized =
        Vec::with_capacity(8 + ProposalTransaction::get_size(asset_len, instructions_total_size));
    data.try_serialize(&mut serialized)?;
    let rent = svm.minimum_balance_for_rent_exemption(serialized.len());
    let account = Account {
        lamports: rent,
        data: serialized,
        owner: MULTISIG_PROGRAM_ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(proposal_transaction, account)?;
    Ok(())
}

pub fn close_proposal_account(svm: &mut LiteSVM, proposal: Pubkey) -> Result<()> {
    let account = Account {
        lamports: 0,
        data: Vec::new(),
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    };
    svm.set_account(proposal, account)?;
    Ok(())
}

/// Marks a created config proposal as expired.
///
/// Sets state to Passed (with passed_timestamp 0) and sets the deadline to -1,
/// which is expired relative to LiteSVM's clock that starts at 0.
pub fn set_config_proposal_as_expired(svm: &mut LiteSVM, proposal: Pubkey) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = ConfigProposal::try_deserialize(&mut data)?;
    proposal_data.set_state(ProposalState::Passed)?;
    proposal_data.set_proposal_passed_timestamp(0);
    proposal_data.timelock_offset = 0;
    proposal_data.proposal_deadline_timestamp = -1;
    let mut serialized = Vec::with_capacity(account.data.len());
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

/// Marks a created config proposal as still timelocked.
///
/// Sets state to Passed with passed_timestamp=0 and a huge timelock_offset so
/// valid_from = 0 + (u32::MAX - 1), in the LiteSVM future.
pub fn set_config_proposal_as_timelocked(svm: &mut LiteSVM, proposal: Pubkey) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = ConfigProposal::try_deserialize(&mut data)?;
    proposal_data.set_state(ProposalState::Passed)?;
    proposal_data.set_proposal_passed_timestamp(0);
    proposal_data.timelock_offset = u32::MAX - 1;
    let mut serialized = Vec::with_capacity(account.data.len());
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

/// Marks a created config proposal as stale.
///
/// Sets the proposal to Passed state and then bumps the group's
/// `proposal_index_after_stale` to u64::MAX so every existing proposal is stale.
pub fn mark_config_proposal_as_stale(
    svm: &mut LiteSVM,
    group: Pubkey,
    proposal: Pubkey,
) -> Result<()> {
    set_config_proposal_state(svm, proposal, ProposalState::Passed, Some(0))?;
    let mut account = svm
        .get_account(&group)
        .ok_or_else(|| anyhow::anyhow!("group account not found"))?;
    let mut data = account.data.as_slice();
    let mut group_data = Group::try_deserialize(&mut data)?;
    group_data.proposal_index_after_stale = u64::MAX;
    let mut serialized = Vec::with_capacity(account.data.len());
    group_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(group, account)?;
    Ok(())
}

/// Patch the `proposal_index_after_stale` on the group account to the given value.
/// Setting it to a value greater than any existing proposal index makes all those
/// proposals stale without changing their own state.
pub fn set_group_stale_after_index(
    svm: &mut LiteSVM,
    group: Pubkey,
    stale_from_index: u64,
) -> Result<()> {
    let mut account = svm
        .get_account(&group)
        .ok_or_else(|| anyhow::anyhow!("group account not found"))?;
    let mut data = account.data.as_slice();
    let mut group_data = Group::try_deserialize(&mut data)?;
    group_data.proposal_index_after_stale = stale_from_index;
    let mut serialized = Vec::with_capacity(account.data.len());
    group_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(group, account)?;
    Ok(())
}

/// Read and deserialize the on-chain Group account.
pub fn read_group(svm: &LiteSVM, group: Pubkey) -> Result<Group> {
    let account = svm
        .get_account(&group)
        .ok_or_else(|| anyhow::anyhow!("group account not found"))?;
    let mut data = account.data.as_slice();
    Ok(Group::try_deserialize(&mut data)?)
}

/// Patch the `permissions` field of a GroupMember PDA.
/// Useful for testing permission-gated instructions without re-creating the group.
pub fn set_group_member_permissions(
    svm: &mut LiteSVM,
    group: Pubkey,
    member: Pubkey,
    permissions: Permissions,
) -> Result<()> {
    let member_pda = get_group_member(&group, &member);
    let mut account = svm
        .get_account(&member_pda)
        .ok_or_else(|| anyhow::anyhow!("group member account not found"))?;
    let mut data = account.data.as_slice();
    let mut member_data = GroupMember::try_deserialize(&mut data)?;
    member_data.permissions = permissions;
    let mut serialized = Vec::with_capacity(account.data.len());
    member_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(member_pda, account)?;
    Ok(())
}

/// Patch the `weight` field of a GroupMember PDA.
/// Setting weight to 0 turns the member into an unauthorized voter.
pub fn set_group_member_weight(
    svm: &mut LiteSVM,
    group: Pubkey,
    member: Pubkey,
    weight: u32,
) -> Result<()> {
    let member_pda = get_group_member(&group, &member);
    let mut account = svm
        .get_account(&member_pda)
        .ok_or_else(|| anyhow::anyhow!("group member account not found"))?;
    let mut data = account.data.as_slice();
    let mut member_data = GroupMember::try_deserialize(&mut data)?;
    member_data.weight = weight;
    let mut serialized = Vec::with_capacity(account.data.len());
    member_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(member_pda, account)?;
    Ok(())
}

/// Set a config proposal's deadline to `deadline` without changing proposal state.
/// LiteSVM clock starts at 0, so using -1 makes an Open proposal immediately expired.
pub fn set_config_proposal_deadline(
    svm: &mut LiteSVM,
    proposal: Pubkey,
    deadline: i64,
) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = ConfigProposal::try_deserialize(&mut data)?;
    proposal_data.proposal_deadline_timestamp = deadline;
    let mut serialized = Vec::with_capacity(account.data.len());
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

/// Set a normal proposal's deadline to `deadline` without changing proposal state.
pub fn set_normal_proposal_deadline(
    svm: &mut LiteSVM,
    proposal: Pubkey,
    deadline: i64,
) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = NormalProposal::try_deserialize(&mut data)?;
    proposal_data.proposal_deadline_timestamp = deadline;
    let mut serialized = Vec::with_capacity(account.data.len());
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

/// Marks a normal proposal as Passed with timelock_offset = u32::MAX - 1.
/// valid_from = 0 + (u32::MAX - 1) is in the LiteSVM future.
pub fn set_normal_proposal_as_timelocked(svm: &mut LiteSVM, proposal: Pubkey) -> Result<()> {
    let mut account = svm
        .get_account(&proposal)
        .ok_or_else(|| anyhow::anyhow!("proposal account not found"))?;
    let mut data = account.data.as_slice();
    let mut proposal_data = NormalProposal::try_deserialize(&mut data)?;
    proposal_data.set_state(ProposalState::Passed)?;
    proposal_data.set_proposal_passed_timestamp(0);
    proposal_data.timelock_offset = u32::MAX - 1;
    let mut serialized = Vec::with_capacity(account.data.len());
    proposal_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(proposal, account)?;
    Ok(())
}

/// Directly patch `group.minimum_timelock` in the on-chain account.
/// Used in tests to set an enforced minimum without going through a full proposal.
pub fn set_group_minimum_timelock(
    svm: &mut LiteSVM,
    group: Pubkey,
    minimum_timelock: u32,
) -> Result<()> {
    let mut account = svm
        .get_account(&group)
        .ok_or_else(|| anyhow::anyhow!("group account not found"))?;
    let mut data = account.data.as_slice();
    let mut group_data = Group::try_deserialize(&mut data)?;
    group_data.minimum_timelock = minimum_timelock;
    let mut serialized = Vec::with_capacity(account.data.len());
    group_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(group, account)?;
    Ok(())
}

/// Directly set `group.paused` and the three trusted keys in the on-chain account.
/// Used in pause-mode tests to skip the full emergency-reset proposal flow.
pub fn set_group_paused(
    svm: &mut LiteSVM,
    group: Pubkey,
    paused: bool,
    trusted_1: Pubkey,
    trusted_2: Pubkey,
    trusted_3: Pubkey,
) -> Result<()> {
    let mut account = svm
        .get_account(&group)
        .ok_or_else(|| anyhow::anyhow!("group account not found"))?;
    let mut data = account.data.as_slice();
    let mut group_data = Group::try_deserialize(&mut data)?;
    group_data.paused = paused;
    group_data.reset_trusted_1 = trusted_1;
    group_data.reset_trusted_2 = trusted_2;
    group_data.reset_trusted_3 = trusted_3;
    let mut serialized = Vec::with_capacity(account.data.len());
    group_data.try_serialize(&mut serialized)?;
    account.data = serialized;
    svm.set_account(group, account)?;
    Ok(())
}

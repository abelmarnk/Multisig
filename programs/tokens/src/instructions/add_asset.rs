use anchor_lang::{
    prelude::*,
    solana_program::program_option::COption
};
use anchor_spl::token_interface::{
    TokenInterface,
    TokenAccount, Mint,
    spl_token_2022::{
        state::AccountState
    }
};

use crate::{Permissions, utils::fractional_threshold::FractionalThreshold};

use crate::state::{
    asset::Asset,
    group::Group,
    member::{
        AssetMember,
        GroupMember
    },
    error::TokenError,
};

// Instruction arguments struct for AddAssetMintInstructionAccounts
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddAssetMintInstructionArgs {
    pub member_key_1: Pubkey,
    pub member_key_2: Pubkey,
    pub member_key_3: Pubkey,
    pub initial_weights: [u32; 3],
    pub initial_permissions: [Permissions; 3],
    pub use_threshold: FractionalThreshold,
    pub not_use_threshold: FractionalThreshold,
    pub add_threshold: FractionalThreshold,
    pub not_add_threshold: FractionalThreshold,
    pub remove_threshold: FractionalThreshold,
    pub not_remove_threshold: FractionalThreshold,
    pub change_config_threshold: FractionalThreshold,
    pub not_change_config_threshold: FractionalThreshold,
    pub minimum_member_count: u32,
    pub minimum_vote_count: u32,
}

// Instruction arguments struct for AddAssetTokenInstructionAccounts
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddAssetTokenInstructionArgs {
    pub member_key_1: Pubkey,
    pub member_key_2: Pubkey,
    pub member_key_3: Pubkey,
    pub initial_weights: [u32; 3],
    pub initial_permissions: [Permissions; 3],
    pub use_threshold: FractionalThreshold,
    pub not_use_threshold: FractionalThreshold,
    pub add_threshold: FractionalThreshold,
    pub not_add_threshold: FractionalThreshold,
    pub remove_threshold: FractionalThreshold,
    pub not_remove_threshold: FractionalThreshold,
    pub change_config_threshold: FractionalThreshold,
    pub not_change_config_threshold: FractionalThreshold,
    pub minimum_member_count: u32,
    pub minimum_vote_count: u32,
}

#[derive(Accounts)]
#[instruction(args: AddAssetMintInstructionArgs)]
pub struct AddAssetMintInstructionAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + Asset::INIT_SPACE,
        seeds = [b"asset", group.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub asset: Account<'info, Asset>,

    #[account(
        seeds = [b"authority", group.key().as_ref(), mint.key().as_ref()],
        bump
    )]
    pub asset_authority: UncheckedAccount<'info>,

    #[account(
        seeds = [b"member", group.key().as_ref(), payer.key.as_ref()],
        bump = adder.get_account_bump()
    )]
    pub adder: Account<'info, GroupMember>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_1.as_ref()],
        bump = group_member_1.get_account_bump()
    )]
    pub group_member_1: Account<'info, GroupMember>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_2.as_ref()],
        bump = group_member_2.get_account_bump()
    )]
    pub group_member_2: Account<'info, GroupMember>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_3.as_ref()],
        bump = group_member_3.get_account_bump()
    )]
    pub group_member_3: Account<'info, GroupMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), mint.key().as_ref(), args.member_key_1.as_ref()],
        bump
    )]
    pub asset_member_1: Account<'info, AssetMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), mint.key().as_ref(), args.member_key_2.as_ref()],
        bump
    )]
    pub asset_member_2: Account<'info, AssetMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), mint.key().as_ref(), args.member_key_3.as_ref()],
        bump
    )]
    pub asset_member_3: Account<'info, AssetMember>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn add_asset_mint_handler(
    ctx: Context<AddAssetMintInstructionAccounts>,
    args: AddAssetMintInstructionArgs,
) -> Result<()> {
    // Destructure the arguments at the beginning
    let AddAssetMintInstructionArgs {
        member_key_1,
        member_key_2,
        member_key_3,
        initial_weights,
        initial_permissions,
        use_threshold,
        not_use_threshold,
        add_threshold,
        not_add_threshold,
        remove_threshold,
        not_remove_threshold,
        change_config_threshold,
        not_change_config_threshold,
        minimum_member_count,
        minimum_vote_count,
    } = args;

    let adder = &ctx.accounts.adder;

    // Ensure adder has the add_asset permission
    require!(adder.has_add_asset(), TokenError::InsufficientPermissions);

    // Initialize the Asset account
    let mint_key = ctx.accounts.mint.key();
    let asset_acc = &mut ctx.accounts.asset;

    asset_acc.set_inner(Asset::new(
        mint_key,
        use_threshold,
        not_use_threshold,
        add_threshold,
        not_add_threshold,
        remove_threshold,
        not_remove_threshold,
        change_config_threshold,
        not_change_config_threshold,
        minimum_member_count,
        minimum_vote_count,
        ctx.bumps.asset,
        ctx.bumps.asset_authority,
    )?);

    // Initialize AssetMember accounts
    let member_keys = [member_key_1, member_key_2, member_key_3];
    let member_bumps = [
        ctx.bumps.asset_member_1,
        ctx.bumps.asset_member_2,
        ctx.bumps.asset_member_3,
    ];
    let mut asset_members: [&mut Account<AssetMember>; 3] = [
        &mut ctx.accounts.asset_member_1,
        &mut ctx.accounts.asset_member_2,
        &mut ctx.accounts.asset_member_3,
    ];

    for ((((asset_member, key), bump), weight), permissions) in asset_members
        .iter_mut()
        .zip(member_keys.into_iter())
        .zip(member_bumps.into_iter())
        .zip(initial_weights.into_iter())
        .zip(initial_permissions.into_iter())
    {
        (*asset_member).set_inner(AssetMember::new(
            key,
            *asset_acc.get_asset_address(),
            permissions,
            weight,
            bump,
            ctx.accounts.group.get_max_member_weight(),
        )?);
    }

    // Token authority checks (mint/freeze)
    
    require_keys_eq!(
        ctx.accounts.mint.mint_authority.ok_or(TokenError::AuthorityNotProvided)?,
        *ctx.accounts.asset_authority.key,
        TokenError::InvalidMintMintAuthority
    );

    match ctx.accounts.mint.freeze_authority.as_ref() {
            COption::Some(freeze_authority) => {
                require_keys_eq!(
                    *freeze_authority,
                    *ctx.accounts.asset_authority.key,
                    TokenError::InvalidMintMintAuthority
                );
            }
            COption::None => {}
        }

    Ok(())
}

#[derive(Accounts)]
#[instruction(args: AddAssetTokenInstructionArgs)]
pub struct AddAssetTokenInstructionAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    pub token: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [b"member", group.key().as_ref(), payer.key.as_ref()],
        bump = adder.get_account_bump()
    )]
    pub adder: Account<'info, GroupMember>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_1.as_ref()],
        bump = group_member_1.get_account_bump()
    )]
    pub group_member_1: Account<'info, GroupMember>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_2.as_ref()],
        bump = group_member_2.get_account_bump()
    )]
    pub group_member_2: Account<'info, GroupMember>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_3.as_ref()],
        bump = group_member_3.get_account_bump()
    )]
    pub group_member_3: Account<'info, GroupMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + Asset::INIT_SPACE,
        seeds = [b"asset", group.key().as_ref(), token.key().as_ref()],
        bump
    )]
    pub asset: Account<'info, Asset>,

    #[account(
        seeds = [b"authority", group.key().as_ref(), token.key().as_ref()],
        bump
    )]
    pub asset_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), token.key().as_ref(), args.member_key_1.as_ref()],
        bump
    )]
    pub asset_member_1: Account<'info, AssetMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), token.key().as_ref(), args.member_key_2.as_ref()],
        bump
    )]
    pub asset_member_2: Account<'info, AssetMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), token.key().as_ref(), args.member_key_3.as_ref()],
        bump
    )]
    pub asset_member_3: Account<'info, AssetMember>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn add_asset_token_handler(
    ctx: Context<AddAssetTokenInstructionAccounts>,
    args: AddAssetTokenInstructionArgs,
) -> Result<()> {
    // Destructure the arguments at the beginning
    let AddAssetTokenInstructionArgs {
        member_key_1,
        member_key_2,
        member_key_3,
        initial_weights,
        initial_permissions,
        use_threshold,
        not_use_threshold,
        add_threshold,
        not_add_threshold,
        remove_threshold,
        not_remove_threshold,
        change_config_threshold,
        not_change_config_threshold,
        minimum_member_count,
        minimum_vote_count,
    } = args;

    // Ensure adder has the add_asset permission
    let adder = &ctx.accounts.adder;
    require!(adder.has_add_asset(), TokenError::InsufficientPermissions);

    // Initialize Asset
    let token_key = ctx.accounts.token.key();
    let asset_acc = &mut ctx.accounts.asset;

    asset_acc.set_inner(Asset::new(
        token_key,
        use_threshold,
        not_use_threshold,
        add_threshold,
        not_add_threshold,
        remove_threshold,
        not_remove_threshold,
        change_config_threshold,
        not_change_config_threshold,
        minimum_member_count,
        minimum_vote_count,
        ctx.bumps.asset,
        ctx.bumps.asset_authority,
    )?);

    // Initialize AssetMembers
    let member_keys = [member_key_1, member_key_2, member_key_3];
    let member_bumps = [
        ctx.bumps.asset_member_1,
        ctx.bumps.asset_member_2,
        ctx.bumps.asset_member_3,
    ];
    let mut asset_members: [&mut Account<AssetMember>; 3] = [
        &mut ctx.accounts.asset_member_1,
        &mut ctx.accounts.asset_member_2,
        &mut ctx.accounts.asset_member_3,
    ];

    for ((((asset_member, key), bump), weight), permissions) in asset_members
        .iter_mut()
        .zip(member_keys.into_iter())
        .zip(member_bumps.into_iter())
        .zip(initial_weights.into_iter())
        .zip(initial_permissions.into_iter())
    {
        (*asset_member).set_inner(AssetMember::new(
            key,
            *asset_acc.get_asset_address(),
            permissions,
            weight,
            bump,
            ctx.accounts.group.get_max_member_weight(),
        )?);
    }

    // Token account must be initialized
    require!(
        ctx.accounts.token.state == AccountState::Initialized,
        TokenError::InvalidAccountState
    );

    // Owner must equal asset_authority
    require_keys_eq!(
        ctx.accounts.token.owner,
        *ctx.accounts.asset_authority.key,
        TokenError::InvalidTokenOwner
    );

    // Delegate must be None
    require!(ctx.accounts.token.delegate.is_none(), TokenError::InvalidTokenDelegate);

    // Close authority must be None or asset_authority
    match ctx.accounts.token.close_authority {
        COption::Some(close_auth) => {
            require_keys_eq!(
                close_auth,
                *ctx.accounts.asset_authority.key,
                TokenError::InvalidCloseAuthority
            );
        }
        COption::None => {}
    }

    Ok(())
}
use anchor_lang::{prelude::*, solana_program::program_option::COption};
use anchor_spl::{
    token::ID as TOKEN_PROGRAM_ID,
    token_interface::{
        spl_token_2022::{
            self,
            extension::{BaseStateWithExtensions, StateWithExtensions},
            state::{Account as Token2022Account, AccountState},
        },
        TokenAccount, TokenInterface,
    },
};

use crate::{utils::fractional_threshold::FractionalThreshold, Permissions};

use crate::state::{
    asset::Asset,
    error::MultisigError,
    group::Group,
    member::{AssetMember, GroupMember},
};

#[inline(always)]
fn require_supported_token_extensions(
    token: &AccountInfo<'_>,
    token_program: Pubkey,
) -> Result<()> {
    if token_program == TOKEN_PROGRAM_ID {
        return Ok(());
    }

    require_keys_eq!(
        token_program,
        spl_token_2022::ID,
        MultisigError::UnsupportedTokenProgram
    );

    let data = token.data.borrow();
    let token_with_extensions = StateWithExtensions::<Token2022Account>::unpack(&data)?;
    let ext_types = token_with_extensions
        .get_extension_types()
        .map_err(|_| MultisigError::UnsupportedTokenExtensions)?;
    require!(
        ext_types.is_empty(),
        MultisigError::UnsupportedTokenExtensions
    );

    Ok(())
}

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
#[instruction(args: AddAssetTokenInstructionArgs)]
pub struct AddAssetTokenInstructionAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    pub token: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [b"member", group.key().as_ref(), payer.key.as_ref()],
        bump = adder.account_bump
    )]
    pub adder: Account<'info, GroupMember>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_1.as_ref()],
        bump = group_member_1.account_bump
    )]
    pub group_member_1: Box<Account<'info, GroupMember>>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_2.as_ref()],
        bump = group_member_2.account_bump
    )]
    pub group_member_2: Box<Account<'info, GroupMember>>,

    #[account(
        seeds = [b"member", group.key().as_ref(), args.member_key_3.as_ref()],
        bump = group_member_3.account_bump
    )]
    pub group_member_3: Box<Account<'info, GroupMember>>,

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
    /// CHECK: New Asset authority
    pub asset_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), token.key().as_ref(), args.member_key_1.as_ref()],
        bump
    )]
    pub asset_member_1: Box<Account<'info, AssetMember>>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), token.key().as_ref(), args.member_key_2.as_ref()],
        bump
    )]
    pub asset_member_2: Box<Account<'info, AssetMember>>,

    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), token.key().as_ref(), args.member_key_3.as_ref()],
        bump
    )]
    pub asset_member_3: Box<Account<'info, AssetMember>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(ctx: &Context<AddAssetTokenInstructionAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    let adder = &ctx.accounts.adder;
    require!(
        adder.has_add_asset(),
        MultisigError::InsufficientPermissions
    );

    require_supported_token_extensions(
        &ctx.accounts.token.to_account_info(),
        ctx.accounts.token_program.key(),
    )?;

    require!(
        ctx.accounts.token.state == AccountState::Initialized,
        MultisigError::InvalidAccountState
    );

    require_keys_eq!(
        ctx.accounts.token.owner,
        *ctx.accounts.asset_authority.key,
        MultisigError::InvalidTokenOwner
    );

    require!(
        ctx.accounts.token.delegate.is_none(),
        MultisigError::InvalidTokenDelegate
    );

    match ctx.accounts.token.close_authority {
        COption::Some(close_auth) => {
            require_keys_eq!(
                close_auth,
                *ctx.accounts.asset_authority.key,
                MultisigError::InvalidCloseAuthority
            );
        }
        COption::None => {} // Ok
    }

    Ok(())
}

/// Registers a new token account that is controlled by the multisig.
pub fn add_asset_token_handler(
    ctx: Context<AddAssetTokenInstructionAccounts>,
    args: AddAssetTokenInstructionArgs,
) -> Result<()> {
    checks(&ctx)?;

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
        3,
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
            ctx.accounts.group.key(),
            asset_acc.asset_address,
            permissions,
            weight,
            bump,
            ctx.accounts.group.max_member_weight,
        )?);
    }

    Ok(())
}

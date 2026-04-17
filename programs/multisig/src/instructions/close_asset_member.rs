use crate::state::{asset::Asset, error::MultisigError, group::Group, member::AssetMember};
use anchor_lang::{prelude::*, solana_program::system_program};

#[derive(Accounts)]
pub struct CleanUpAssetMemberInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// Seeds [group, asset_member.asset, member] guarantee asset_member.{group, asset, user}
    /// all match the other accounts - no separate field equality checks are needed.
    #[account(
        mut,
        seeds = [
            b"asset-member",
            group.key().as_ref(),
            asset_member.asset.as_ref(),
            member.key().as_ref()
        ],
        bump = asset_member.account_bump,
        close = rent_collector
    )]
    pub asset_member: Account<'info, AssetMember>,

    /// Seeds bind asset to group + asset_member.asset - asset.group == group and
    /// asset.asset_address == asset_member.asset are guaranteed.
    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset_member.asset.as_ref()],
        bump = asset.account_bump
    )]
    pub asset: Account<'info, Asset>,

    /// CHECK: Seeds bind this PDA to group + member; we only verify it is absent (closed).
    #[account(
        seeds = [b"member", group.key().as_ref(), member.key().as_ref()],
        bump,
    )]
    pub group_member: UncheckedAccount<'info>,

    /// CHECK: Only the key is used as a seed for the asset-member and group-member PDAs.
    pub member: UncheckedAccount<'info>,

    /// CHECK: Rent collector; verified against group.rent_collector in checks().
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<CleanUpAssetMemberInstructionAccounts>) -> Result<()> {
    let group = &ctx.accounts.group;
    let group_member = &ctx.accounts.group_member;
    let rent_collector = &ctx.accounts.rent_collector;

    require_keys_eq!(
        rent_collector.key(),
        group.rent_collector,
        MultisigError::UnexpectedRentCollector
    );

    require!(
        group_member.owner == &system_program::ID && group_member.data_is_empty(),
        MultisigError::GroupMemberStillActive
    );

    Ok(())
}

/// Close an asset member account that has had it's group member account removed(by a proposal).
/// The rent is sent to the rent collector.
/// This instruction can be called by anyone.
pub fn clean_up_asset_member_handler(
    ctx: Context<CleanUpAssetMemberInstructionAccounts>,
) -> Result<()> {
    checks(&ctx)?;
    ctx.accounts.asset.decrement_member_count()
}

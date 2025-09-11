use crate::state::{
    error::MultisigError,
    group::Group,
    member::AssetMember,
};
use anchor_lang::{prelude::*, solana_program::system_program};

#[derive(Accounts)]
// The accounts are bound together with the seeds
pub struct CloseAssetMemberInstructionAccounts<'info> {
    /// The group that this asset member belongs to
    #[account(
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// The asset member account we want to close
    #[account(
        mut,
        seeds = [b"asset_member", group.key().as_ref(), member.key().as_ref()],
        bump = asset_member.get_account_bump(),
        close = rent_collector
    )]
    pub asset_member: Account<'info, AssetMember>,

    /// The corresponding group member account.
    /// CHECK: We only verify its absence (it must have been closed).
    #[account(
        seeds = [b"member", group.key().as_ref(), member.key().as_ref()],
        bump,
    )]
    pub group_member: UncheckedAccount<'info>,

    pub member:SystemAccount<'info>,

    /// Collector of the `asset_member` rent
    #[account(mut)]
    /// CHECK: open to anyone right now, later canonicalized
    pub rent_collector: UncheckedAccount<'info>,
}

pub fn close_asset_member_handler(
    ctx: Context<CloseAssetMemberInstructionAccounts>,
) -> Result<()> {
    let group_member = &ctx.accounts.group_member;


    // Ensure that the group member account is *closed*
    // A closed account will have lamports == 0 and owner == system_program and data empty
    require!(
        group_member.lamports() == 0 && 
        group_member.owner == &system_program::ID && 
        group_member.data_is_empty(),
        MultisigError::GroupMemberStillActive
    );

    Ok(())
}

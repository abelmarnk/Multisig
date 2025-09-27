use crate::state::{
    error::MultisigError,
    group::Group,
    member::AssetMember,
};
use anchor_lang::{prelude::*, solana_program::system_program};

#[derive(Accounts)]
/// The accounts are bound together with the seeds and the rent collector check in the handler
pub struct CleanUpAssetMemberInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

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

    /// CHECK: Rent collector
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,
}

#[inline(always)] /// This function is only called once in the handler
fn clean_up_asset_member_checks(
    ctx: &Context<CleanUpAssetMemberInstructionAccounts>
)->Result<()>{
    let group = &ctx.accounts.group;
    let group_member = &ctx.accounts.group_member;
    let rent_collector = &ctx.accounts.rent_collector;

    // Validate rent collector
    require_keys_eq!(
        rent_collector.key(),
        *group.get_rent_collector(),
        MultisigError::UnexpectedRentCollector
    );

    // Ensure that the group member account is closed
    // A closed account will have lamports == 0 and owner == system_program and data empty
    require!(
        group_member.lamports() == 0 && 
        group_member.owner == &system_program::ID && 
        group_member.data_is_empty(),
        MultisigError::GroupMemberStillActive
    );

    Ok(())
}

/// Close an asset member account that has had it's group member account removed(by a proposal)
/// the rent is sent to the rent collector
/// This instruction can be called by anyone
pub fn clean_up_asset_member_handler(
    ctx: Context<CleanUpAssetMemberInstructionAccounts>,
) -> Result<()> {
    clean_up_asset_member_checks(&ctx)
}

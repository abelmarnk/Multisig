use anchor_lang::prelude::*;

use crate::state::{error::MultisigError, group::Group, member::GroupMember};

#[derive(Accounts)]
pub struct RemoveMemberInResetModeAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// CHECK: Only used as a seed; authorisation comes from the trusted member triplet.
    pub member: UncheckedAccount<'info>,

    /// Seeds bind member account to group + member.
    #[account(
        mut,
        seeds = [b"member", group.key().as_ref(), member.key().as_ref()],
        bump = member_account.account_bump,
        close = rent_collector,
    )]
    pub member_account: Account<'info, GroupMember>,

    pub trusted_1: Signer<'info>,
    pub trusted_2: Signer<'info>,
    pub trusted_3: Signer<'info>,

    /// CHECK: Validated against group.rent_collector in checks().
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<RemoveMemberInResetModeAccounts>) -> Result<()> {
    require!(ctx.accounts.group.paused, MultisigError::GroupNotPaused);

    require_keys_eq!(
        ctx.accounts.trusted_1.key(),
        ctx.accounts.group.reset_trusted_1,
        MultisigError::InvalidTrustedMember
    );
    require_keys_eq!(
        ctx.accounts.trusted_2.key(),
        ctx.accounts.group.reset_trusted_2,
        MultisigError::InvalidTrustedMember
    );
    require_keys_eq!(
        ctx.accounts.trusted_3.key(),
        ctx.accounts.group.reset_trusted_3,
        MultisigError::InvalidTrustedMember
    );

    require_keys_eq!(
        ctx.accounts.rent_collector.key(),
        ctx.accounts.group.rent_collector,
        MultisigError::UnexpectedRentCollector
    );

    Ok(())
}

/// Removes a group member while paused. All three trusted members must sign.
pub fn remove_member_in_reset_mode_handler(
    ctx: Context<RemoveMemberInResetModeAccounts>,
) -> Result<()> {
    checks(&ctx)?;

    ctx.accounts.group.force_decrement_member_count();

    Ok(())
}

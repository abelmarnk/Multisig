use anchor_lang::prelude::*;

use crate::{
    state::{
        group::Group,
        proposal::{ConfigChange, ConfigProposal, ProposalState},
        MultisigError,
    },
    GroupMember,
};

#[derive(Accounts)]
pub struct RemoveGroupMemberInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"member", group.key().as_ref(), group_member_account.user.as_ref()],
        bump = group_member_account.account_bump,
        close = rent_collector
    )]
    pub group_member_account: Account<'info, GroupMember>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer
    )]
    pub proposal: Account<'info, ConfigProposal>,

    /// CHECK: Validated against group.rent_collector in checks().
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,

    /// CHECK: Must match the proposer stored in the proposal; receives closed-account rent.
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(ctx: &Context<RemoveGroupMemberInstructionAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require_keys_eq!(
        ctx.accounts.rent_collector.key(),
        ctx.accounts.group.rent_collector,
        MultisigError::UnexpectedRentCollector
    );

    require_keys_eq!(
        ctx.accounts.proposer.key(),
        ctx.accounts.proposal.proposer,
        MultisigError::InvalidProposer
    );

    require!(
        ctx.accounts.proposal.state == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    let now = Clock::get()?.unix_timestamp;

    require_gte!(
        now,
        ctx.accounts.proposal.get_valid_from_timestamp()?,
        MultisigError::ProposalStillTimelocked
    );

    require_gte!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        now,
        MultisigError::ProposalExpired
    );

    require_gte!(
        ctx.accounts.proposal.proposal_index,
        ctx.accounts.group.proposal_index_after_stale,
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Executes a passed RemoveGroupMember config proposal.
pub fn remove_group_member_handler(
    ctx: Context<RemoveGroupMemberInstructionAccounts>,
) -> Result<()> {
    checks(&ctx)?;

    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    match &proposal.config_change {
        ConfigChange::RemoveGroupMember {
            member: target_member,
        } => {
            require_keys_eq!(
                *target_member,
                ctx.accounts.group_member_account.user,
                MultisigError::InvalidMember
            );

            group.decrement_member_count()?;
            group.update_stale_proposal_index();
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

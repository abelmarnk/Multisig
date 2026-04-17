use anchor_lang::prelude::*;

use crate::state::{
    error::MultisigError,
    group::Group,
    proposal::{EmergencyResetProposal, ProposalState},
};

#[derive(Accounts)]
pub struct ExecuteEmergencyResetAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// Seeds bind proposal to group.
    #[account(
        mut,
        seeds = [b"emergency-reset", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer,
    )]
    pub proposal: Account<'info, EmergencyResetProposal>,

    /// CHECK: Must match proposal.proposer, verified in checks().
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<ExecuteEmergencyResetAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require_keys_eq!(
        ctx.accounts.proposer.key(),
        ctx.accounts.proposal.proposer,
        MultisigError::InvalidProposer
    );

    require!(
        ctx.accounts.proposal.state == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    require_gte!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        Clock::get()?.unix_timestamp,
        MultisigError::ProposalExpired
    );

    // No stale-index check, emergency resets survive config changes.

    Ok(())
}

/// Executes a passed emergency reset: pauses the group and installs the trusted keys.
pub fn execute_emergency_reset_handler(ctx: Context<ExecuteEmergencyResetAccounts>) -> Result<()> {
    checks(&ctx)?;

    let proposal = &ctx.accounts.proposal;
    let group = &mut ctx.accounts.group;

    group.update_stale_proposal_index();
    group.pause_group(proposal.trusted_members);

    Ok(())
}

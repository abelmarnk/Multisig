use anchor_lang::prelude::*;

use crate::state::{
    error::MultisigError,
    group::Group,
    proposal::{EmergencyResetProposal, ProposalState},
};

#[derive(Accounts)]
pub struct CloseEmergencyResetProposalAccounts<'info> {
    /// Seeds bind group to its seed.
    #[account(
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// Seeds bind proposal to group - proposal.group == group is guaranteed.
    #[account(
        mut,
        seeds = [b"emergency-reset", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer,
    )]
    pub proposal: Account<'info, EmergencyResetProposal>,

    /// CHECK: Must match proposal.proposer; verified in checks(); receives closed-account rent.
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<CloseEmergencyResetProposalAccounts>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        ctx.accounts.proposal.proposer,
        MultisigError::InvalidProposer
    );

    let proposal = &ctx.accounts.proposal;
    let now = Clock::get()?.unix_timestamp;
    let is_expired = now >= proposal.proposal_deadline_timestamp;

    match proposal.state {
        ProposalState::Open | ProposalState::Passed => {
            require!(is_expired, MultisigError::ProposalStillActive);
        }
        ProposalState::Failed | ProposalState::Expired => {}
        ProposalState::Executed => {}
    }

    Ok(())
}

/// Closes an emergency reset proposal, returning rent to the proposer.
pub fn close_emergency_reset_proposal_handler(
    ctx: Context<CloseEmergencyResetProposalAccounts>,
) -> Result<()> {
    checks(&ctx)
}

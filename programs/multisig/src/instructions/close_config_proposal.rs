use anchor_lang::prelude::*;

use crate::{
    state::{
        error::MultisigError,
        proposal::{ConfigProposal, ProposalState},
    },
    Group,
};

#[inline(always)]
fn validate_proposal_closable(
    group: &Account<'_, Group>,
    proposer: Pubkey,
    proposal_proposer: Pubkey,
    proposal_state: ProposalState,
    proposal_deadline_timestamp: i64,
    proposal_index: u64,
) -> Result<()> {
    require_keys_eq!(proposer, proposal_proposer, MultisigError::InvalidProposer);

    let now = Clock::get()?.unix_timestamp;
    let is_stale = group.proposal_index_after_stale > proposal_index;
    let is_expired = now >= proposal_deadline_timestamp;

    match proposal_state {
        ProposalState::Open => {
            require!(is_expired || is_stale, MultisigError::ProposalStillActive)
        }
        ProposalState::Passed => {
            require!(is_stale || is_expired, MultisigError::ProposalStillActive);
        }
        ProposalState::Expired | ProposalState::Failed | ProposalState::Executed => {}
    }

    Ok(())
}

#[derive(Accounts)]
pub struct CloseProposalInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// Seeds bind proposal to group - proposal.group == group is guaranteed.
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    /// CHECK: Must match the proposer stored in the proposal; verified in checks(); receives rent.
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<CloseProposalInstructionAccounts>) -> Result<()> {
    let proposal = &ctx.accounts.proposal;

    validate_proposal_closable(
        &ctx.accounts.group,
        ctx.accounts.proposer.key(),
        proposal.proposer,
        proposal.state,
        proposal.proposal_deadline_timestamp,
        proposal.proposal_index,
    )
}

/// Close a config proposal that failed, expired, or became stale after passing.
/// This instruction can be called by anyone.
pub fn close_proposal_handler(ctx: Context<CloseProposalInstructionAccounts>) -> Result<()> {
    checks(&ctx)
}

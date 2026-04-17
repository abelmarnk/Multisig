use anchor_lang::prelude::*;

use crate::state::{
    error::MultisigError,
    group::Group,
    proposal::{EmergencyResetProposal, ProposalState},
    vote::VoteRecord,
};

#[derive(Accounts)]
pub struct CloseEmergencyResetVoteRecordAccounts<'info> {
    /// Seeds bind group to its seed.
    #[account(
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// CHECK: May already be closed; checked below.
    pub proposal: UncheckedAccount<'info>,

    /// Seeds bind vote record to group + proposal + voter.
    #[account(
        mut,
        seeds = [b"vote-record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref()],
        bump = vote_record.account_bump,
        close = voter,
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<CloseEmergencyResetVoteRecordAccounts>) -> Result<()> {
    let proposal = &ctx.accounts.proposal;

    if !(proposal.data_is_empty() && proposal.owner == &System::id()) {
        let data = proposal.data.borrow();
        let proposal_account = EmergencyResetProposal::try_deserialize(&mut &data[..])?;

        let now = Clock::get()?.unix_timestamp;
        let is_expired = now >= proposal_account.proposal_deadline_timestamp;

        match proposal_account.state {
            ProposalState::Open => {
                require!(is_expired, MultisigError::ProposalStillActive);
            }
            ProposalState::Passed => {
                require!(is_expired, MultisigError::ProposalStillActive);
            }
            ProposalState::Failed | ProposalState::Expired | ProposalState::Executed => {}
        }
    }

    Ok(())
}

/// Close a vote record for an emergency-reset proposal, returning rent to the voter.
pub fn close_emergency_reset_vote_record_handler(
    ctx: Context<CloseEmergencyResetVoteRecordAccounts>,
) -> Result<()> {
    checks(&ctx)
}

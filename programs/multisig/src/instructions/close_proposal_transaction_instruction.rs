use crate::{
    state::{error::MultisigError, ProposalTransaction},
    Group, NormalProposal, ProposalState,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CloseProposalTransactionInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// CHECK: The proposal related to this transaction; may be closed so we load it as UncheckedAccount.
    pub proposal: UncheckedAccount<'info>,

    /// Seeds bind transaction to proposal - proposal_transaction.proposal == proposal is guaranteed.
    /// proposal_transaction.group is checked against group in checks() because proposal is
    /// UncheckedAccount (no seed chain from transaction -> group).
    #[account(
        mut,
        seeds = [b"proposal-transaction", proposal.key.as_ref()],
        bump = proposal_transaction.account_bump,
        close = rent_collector,
    )]
    pub proposal_transaction: Account<'info, ProposalTransaction>,

    /// CHECK: Rent collector; verified against group.rent_collector in checks().
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<CloseProposalTransactionInstructionAccounts>) -> Result<()> {
    let group = &ctx.accounts.group;
    let rent_collector = &ctx.accounts.rent_collector;
    let proposal_transaction = &ctx.accounts.proposal_transaction;

    require_keys_eq!(
        proposal_transaction.group,
        group.key(),
        MultisigError::UnexpectedGroup
    );

    require_keys_eq!(
        *rent_collector.key,
        group.rent_collector,
        MultisigError::UnexpectedRentCollector
    );

    let proposal = &ctx.accounts.proposal;

    if !(proposal.data_is_empty() && proposal.owner == &System::id()) {
        let proposal_account: NormalProposal =
            NormalProposal::try_deserialize(&mut &proposal.data.borrow()[..])?;

        // Ensure the proposal is in a state that allows closing the transaction

        let now = Clock::get()?.unix_timestamp;
        let is_stale = group.proposal_index_after_stale > proposal_transaction.proposal_index;
        let is_expired = now >= proposal_account.proposal_deadline_timestamp;

        match proposal_account.state {
            ProposalState::Open => {
                require!(is_expired || is_stale, MultisigError::ProposalStillActive)
            }
            ProposalState::Passed => {
                require!(is_stale || is_expired, MultisigError::ProposalStillActive);
            }
            ProposalState::Expired | ProposalState::Failed | ProposalState::Executed => {} // Ok
        }
    }

    Ok(())
}

/// Close a proposal transaction that though was finalized after the proposal was passed
/// and active(no config had changed), execution was delayed till after a config changed
/// and refund the rent to the proposal.
/// This instruction can be called by anyone.
pub fn close_proposal_transaction_handler(
    ctx: Context<CloseProposalTransactionInstructionAccounts>,
) -> Result<()> {
    checks(&ctx)
}

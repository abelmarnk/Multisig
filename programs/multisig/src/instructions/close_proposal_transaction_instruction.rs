use crate::state::{error::MultisigError, group::Group, ProposalTransaction};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CloseProposalTransactionInstructionAccounts<'info> {
    pub group: Account<'info, Group>,

    #[account(
        mut,
        close = rent_collector
    )]
    pub proposal_transaction: Account<'info, ProposalTransaction>,

    /// CHECK: Any account may temporarily collect rent; 
    /// will later be standardized to group rent collector.
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,
}

pub fn close_proposal_transaction_handler(
    ctx: Context<CloseProposalTransactionInstructionAccounts>,
) -> Result<()> {
    let group = &ctx.accounts.group;
    let proposal_transaction = &ctx.accounts.proposal_transaction;

    // Ensure the group matches
    require_keys_eq!(
        proposal_transaction.get_group(),
        group.key(),
        MultisigError::UnexpectedGroup
    );

    // Ensure the proposal has gone stale
    require!(
        proposal_transaction.get_proposal_index() < group.get_proposal_index_after_stale(),
        MultisigError::ProposalNotStale
    );

    Ok(())
}

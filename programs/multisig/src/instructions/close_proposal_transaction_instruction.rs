use anchor_lang::{prelude::*};
use crate::{Group, NormalProposal, ProposalState, state::{ProposalTransaction, error::MultisigError}};

#[derive(Accounts)]
pub struct CloseProposalTransactionInstructionAccounts<'info> {

    #[account(
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group:Account<'info, Group>,

    /// CHECK: The proposal related to this transaction, it might be closed to we don't attempt to load it
    pub proposal:UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"proposal-transaction", proposal.key.as_ref()],
        bump = proposal_transaction.get_account_bump(),
        close = rent_collector,
    )]
    pub proposal_transaction: Account<'info, ProposalTransaction>,
    
    /// CHECK: Rent collector
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,
}

pub fn close_proposal_transaction_checks(
    ctx: &Context<CloseProposalTransactionInstructionAccounts>    
)->Result<()>{
    let group = &ctx.accounts.group;
    let rent_collector = &ctx.accounts.rent_collector;
    let proposal_transaction = &ctx.accounts.proposal_transaction;

    // Ensure the rent collector matches
    require_keys_eq!(
        *rent_collector.key,
        *group.get_rent_collector(),
        MultisigError::UnexpectedRentCollector   
    );

    // Check if the proposal is closed
    let proposal = &ctx.accounts.proposal;

    
    if !(proposal.data_is_empty() && proposal.lamports() == 0 && proposal.owner == &System::id()) {
        let proposal_account: NormalProposal = NormalProposal::try_deserialize(
            &mut &proposal.data.borrow()[..])?;        

        // Ensure the proposal is in a state that allows closing the transaction
        
        match proposal_account.get_state() {
            ProposalState::Open =>{
                let now = Clock::get()?.unix_timestamp;

                require_gt!(
                    now,
                    proposal_account.get_proposal_deadline_timestamp(),
                    MultisigError::ProposalStillActive
                )
            },
            ProposalState::Passed =>{
                require_gt!(
                    group.get_proposal_index_after_stale(),
                    proposal_transaction.get_proposal_index(),
                    MultisigError::ProposalStillActive
                );
            },
            ProposalState::Expired | ProposalState::Failed =>{} // Ok
        }
    }

    Ok(())
}

/// Close a proposal transaction that though was finalized after the proposal was passed
/// and active(no config had changed), execution was delayed till after a config changed
/// and refund the rent to the proposal
/// This instruction can be called by anyone
pub fn close_proposal_transaction_handler(
    ctx: Context<CloseProposalTransactionInstructionAccounts>,
) -> Result<()> {
    close_proposal_transaction_checks(&ctx)
}

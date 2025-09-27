use crate::{Group, state::{error::MultisigError, proposal::{ConfigProposal, ProposalState}}};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CloseProposalInstructionAccounts<'info> {

    #[account(
        seeds = [b"group", proposal.get_group().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"proposal", proposal.get_group().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    #[account(mut)]
    pub proposer: SystemAccount<'info>,
}

#[inline(always)]// This function is only called once in the handler
fn close_proposal_checks(
    ctx: &Context<CloseProposalInstructionAccounts>
)->Result<()>{
    let group = &ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    // Ensure proposer matches
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Ensure the proposal is in a state that allows closing the transaction

    let now = Clock::get()?.unix_timestamp;

    match proposal.get_state() {
        ProposalState::Open =>{
            require_gt!(
                now,
                proposal.get_proposal_deadline_timestamp(),
                MultisigError::ProposalStillActive
            )
        },
        ProposalState::Passed =>{
             require_gt!(
                    group.get_proposal_index_after_stale(),
                    proposal.get_proposal_index(),
                    MultisigError::ProposalStillActive
            );
        },
        ProposalState::Expired | ProposalState::Failed =>{} // Ok
    }

    Ok(())
}

/// Close a proposal that failed or expired and refund the rent to the proposer
/// This instruction can be called by anyone
pub fn close_proposal_handler(
    ctx: Context<CloseProposalInstructionAccounts>,
) -> Result<()> {
    close_proposal_checks(&ctx)
}

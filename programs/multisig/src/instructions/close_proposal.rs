use crate::state::{error::MultisigError, group::Group, proposal::{ConfigProposal, ProposalState}};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CloseProposalInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    #[account(mut)]
    pub proposer: SystemAccount<'info>,
}

pub fn close_proposal_handler(
    ctx: Context<CloseProposalInstructionAccounts>,
) -> Result<()> {
    let proposal = &ctx.accounts.proposal;
    let group = &ctx.accounts.group;

    // Ensure proposer matches
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Ensure proposal has either failed or expired
    let state = proposal.get_state();

    require!(
        state == ProposalState::Failed,
        MultisigError::ProposalNotClosable
    );

    // If it hasn't entered an unusable state check it has becomes stale
    require_gt!(
        group.get_proposal_index_after_stale(),
        proposal.get_proposal_index(),
        MultisigError::ProposalStale
    );

    Ok(())
}

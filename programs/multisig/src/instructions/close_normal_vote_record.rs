use crate::{
    state::{error::MultisigError, group::Group, proposal::ConfigProposal, vote::VoteRecord},
    NormalProposal, ProposalState,
};
use anchor_lang::prelude::*;

#[inline(always)]
fn ensure_proposal_allows_vote_record_close(
    group: &Account<'_, Group>,
    proposal_state: ProposalState,
    proposal_deadline_timestamp: i64,
    proposal_index: u64,
) -> Result<()> {
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CloseNormalVoteRecordInstructionArgs {
    pub asset_index: u8,
}

#[derive(Accounts)]
#[instruction(args: CloseNormalVoteRecordInstructionArgs)]
pub struct CloseNormalVoteRecordInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// CHECK: May already be closed, we make the necessary checks in the handler.
    pub proposal: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"vote-record", group.key().as_ref(), proposal.key().as_ref(),
                voter.key().as_ref(), &[args.asset_index]],
        bump = vote_record.account_bump,
        close = voter
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,
}

#[inline(always)]
fn checks(
    ctx: &Context<CloseNormalVoteRecordInstructionAccounts>,
    args: &CloseNormalVoteRecordInstructionArgs,
) -> Result<()> {
    require!(
        ctx.accounts.vote_record.asset_index == Some(args.asset_index),
        MultisigError::InvalidAssetIndex
    );

    let group = &ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    // Check if the proposal is closed
    if !(proposal.data_is_empty() && proposal.owner == &System::id()) {
        let data = proposal.data.borrow();

        if let Ok(proposal_account) = NormalProposal::try_deserialize(&mut &data[..]) {
            ensure_proposal_allows_vote_record_close(
                group,
                proposal_account.state,
                proposal_account.proposal_deadline_timestamp,
                proposal_account.proposal_index,
            )?;
        } else {
            let proposal_account = ConfigProposal::try_deserialize(&mut &data[..])?;
            ensure_proposal_allows_vote_record_close(
                group,
                proposal_account.state,
                proposal_account.proposal_deadline_timestamp,
                proposal_account.proposal_index,
            )?;
        }
    }

    Ok(())
}

/// Close a vote record for a normal proposal, the rent is refunded to the voter.
/// This instruction can only be called by the voter.
pub fn close_normal_vote_record_handler(
    ctx: Context<CloseNormalVoteRecordInstructionAccounts>,
    args: CloseNormalVoteRecordInstructionArgs,
) -> Result<()> {
    checks(&ctx, &args)
}

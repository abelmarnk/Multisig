use anchor_lang::prelude::*;

use crate::state::{
    error::MultisigError,
    group::Group,
    member::GroupMember,
    proposal::{EmergencyResetProposal, ProposalState},
    vote::{VoteChoice, VoteRecord},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct VoteOnEmergencyResetArgs {
    pub vote: VoteChoice,
}

#[derive(Accounts)]
pub struct VoteOnEmergencyResetAccounts<'info> {
    /// Seeds bind group to its seed.
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
    )]
    pub proposal: Account<'info, EmergencyResetProposal>,

    /// Seeds bind membership to group + voter.
    #[account(
        seeds = [b"member", group.key().as_ref(), voter.key().as_ref()],
        bump = group_member.account_bump,
    )]
    pub group_member: Account<'info, GroupMember>,

    #[account(
        init_if_needed,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [b"vote-record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref()],
        bump,
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(ctx: &Context<VoteOnEmergencyResetAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require!(
        ctx.accounts.proposal.state == ProposalState::Open,
        MultisigError::ProposalNotOpen
    );

    let now = Clock::get()?.unix_timestamp;
    require_gt!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        now,
        MultisigError::ProposalExpired
    );

    // No stale-index check, emergency resets survive config changes.

    Ok(())
}

/// Votes on an emergency reset proposal.
///
/// Passes when `for_count == member_count`; fails when `against_count == member_count`.
pub fn vote_on_emergency_reset_handler(
    ctx: Context<VoteOnEmergencyResetAccounts>,
    args: VoteOnEmergencyResetArgs,
) -> Result<()> {
    checks(&ctx)?;

    let VoteOnEmergencyResetArgs { vote } = args;

    let proposal = &mut ctx.accounts.proposal;
    let group = &ctx.accounts.group;
    let voter = &ctx.accounts.voter;
    let vote_record = &mut ctx.accounts.vote_record;

    let is_first_vote = !vote_record.is_initialized();
    if !is_first_vote && vote_record.vote_choice == vote {
        return Ok(());
    }

    if is_first_vote {
        proposal.vote_count = proposal
            .vote_count
            .checked_add(1)
            .ok_or(MultisigError::TooManyVotes)?;
    } else {
        match vote_record.vote_choice {
            VoteChoice::For => {
                proposal.for_count = proposal.for_count.saturating_sub(1);
            }
            VoteChoice::Against => {
                proposal.against_count = proposal.against_count.saturating_sub(1);
            }
        }
    }

    match vote {
        VoteChoice::For => {
            proposal.for_count = proposal
                .for_count
                .checked_add(1)
                .ok_or(MultisigError::TooManyVotes)?;
        }
        VoteChoice::Against => {
            proposal.against_count = proposal
                .against_count
                .checked_add(1)
                .ok_or(MultisigError::TooManyVotes)?;
        }
    }

    let member_count = group.member_count;

    if proposal.for_count == member_count {
        proposal.set_state(ProposalState::Passed)?;
    } else if proposal.against_count == member_count {
        proposal.set_state(ProposalState::Failed)?;
    }

    // Record the vote
    if is_first_vote {
        vote_record.set_inner(VoteRecord::new(
            voter.key(),
            proposal.key(),
            None,
            ctx.bumps.vote_record,
            vote,
        ));
    } else {
        vote_record.vote_choice = vote;
    }

    Ok(())
}

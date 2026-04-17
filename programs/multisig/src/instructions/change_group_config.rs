use crate::state::error::*;
use crate::state::{
    group::Group,
    proposal::{ConfigChange, ConfigProposal, ConfigType, ProposalState},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ChangeGroupConfigInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    /// CHECK: Must match the proposer stored in the proposal; receives closed-account rent.
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<ChangeGroupConfigInstructionAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require_keys_eq!(
        ctx.accounts.proposer.key(),
        ctx.accounts.proposal.proposer,
        MultisigError::InvalidProposer
    );

    let now = Clock::get()?.unix_timestamp;

    require_gte!(
        now,
        ctx.accounts.proposal.get_valid_from_timestamp()?,
        MultisigError::ProposalStillTimelocked
    );

    require_gte!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        now,
        MultisigError::ProposalExpired
    );

    require!(
        ctx.accounts.proposal.state == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    require_gte!(
        ctx.accounts.proposal.proposal_index,
        ctx.accounts.group.proposal_index_after_stale,
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Executes a passed ChangeGroupConfig config proposal.
pub fn change_group_config_handler(
    ctx: Context<ChangeGroupConfigInstructionAccounts>,
) -> Result<()> {
    checks(&ctx)?;

    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    group.update_stale_proposal_index();

    match &proposal.config_change {
        ConfigChange::ChangeGroupConfig { config_type } => match config_type {
            ConfigType::AddMember(threshold) => group.set_add_threshold(*threshold)?,
            ConfigType::NotAddMember(threshold) => group.set_not_add_threshold(*threshold)?,
            ConfigType::RemoveMember(threshold) => group.set_remove_threshold(*threshold)?,
            ConfigType::NotRemoveMember(threshold) => group.set_not_remove_threshold(*threshold)?,
            ConfigType::ChangeConfig(threshold) => group.set_change_config_threshold(*threshold)?,
            ConfigType::NotChangeConfig(threshold) => {
                group.set_not_change_config_threshold(*threshold)?
            }
            ConfigType::MinimumMemberCount(count) => group.set_minimum_member_count(*count)?,
            ConfigType::MinimumVoteCount(count) => group.set_minimum_vote_count(*count)?,
            ConfigType::MinimumTimelock(timelock) => group.set_minimum_timelock(*timelock),
            _ => return Err(MultisigError::UnexpectedConfigChange.into()),
        },
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

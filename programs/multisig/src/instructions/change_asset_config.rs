use crate::state::error::*;
use crate::state::{
    asset::Asset,
    group::Group,
    proposal::{ConfigChange, ConfigProposal, ConfigType, ProposalState},
};
use crate::ProposalTarget;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ChangeAssetConfigInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.asset_address.as_ref()],
        bump = asset.account_bump
    )]
    pub asset: Account<'info, Asset>,

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
fn checks(ctx: &Context<ChangeAssetConfigInstructionAccounts>) -> Result<()> {
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

/// Updates asset-wide configuration (e.g, timelock, thresholds, expiry),
/// It must be triggered by an approved proposal and can then be called by anyone.
pub fn change_asset_config_handler(
    ctx: Context<ChangeAssetConfigInstructionAccounts>,
) -> Result<()> {
    checks(&ctx)?;

    let group = &mut ctx.accounts.group;
    let asset = &mut ctx.accounts.asset;
    let proposal = &ctx.accounts.proposal;

    group.update_stale_proposal_index();

    match &proposal.config_change {
        ConfigChange::ChangeAssetConfig { config_type } => {
            let asset_key = match &proposal.target {
                &ProposalTarget::Asset(asset_key) => asset_key,
                ProposalTarget::Group => return Err(MultisigError::InvalidConfigChange.into()),
            };

            require_keys_eq!(asset_key, asset.asset_address, MultisigError::InvalidAsset);

            match config_type {
                ConfigType::AddMember(threshold) => asset.set_add_threshold(*threshold)?,
                ConfigType::NotAddMember(threshold) => asset.set_not_add_threshold(*threshold)?,
                ConfigType::RemoveMember(threshold) => asset.set_remove_threshold(*threshold)?,
                ConfigType::NotRemoveMember(threshold) => {
                    asset.set_not_remove_threshold(*threshold)?
                }
                ConfigType::Use(threshold) => asset.set_use_threshold(*threshold)?,
                ConfigType::NotUse(threshold) => asset.set_not_use_threshold(*threshold)?,
                ConfigType::ChangeConfig(threshold) => {
                    asset.set_change_config_threshold(*threshold)?
                }
                ConfigType::NotChangeConfig(threshold) => {
                    asset.set_not_change_config_threshold(*threshold)?
                }
                ConfigType::MinimumMemberCount(count) => asset.set_minimum_member_count(*count)?,
                ConfigType::MinimumVoteCount(count) => asset.set_minimum_vote_count(*count)?,
                ConfigType::MinimumTimelock(_) => {
                    return Err(MultisigError::InvalidConfigChange.into())
                }
            }
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

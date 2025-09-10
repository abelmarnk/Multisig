use crate::state::error::*;
use crate::state::{
    asset::Asset,
    group::Group,
    proposal::{ConfigChange, ConfigProposal, ConfigType, ProposalState},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ChangeGroupConfigInstructionAccounts<'info> {
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

pub fn change_group_config_handler(
    ctx: Context<ChangeGroupConfigInstructionAccounts>,
) -> Result<()> {
    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *proposal.get_proposer(),
        TokenError::InvalidProposer
    );

    // Validate proposal
    require!(
        proposal.get_state() == ProposalState::Passed,
        TokenError::ProposalNotPassed
    );

    require_gte!(
        proposal.get_proposal_index(),
        group.get_proposal_index_after_stale(),
        TokenError::ProposalStale
    );

    group.set_proposal_index_after_stale(
        proposal
            .get_proposal_index()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    );

    match proposal.get_config_change() {
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
            _ => return Err(TokenError::UnexpectedConfigChange.into()),
        },
        _ => return Err(TokenError::InvalidConfigChange.into()),
    }

    Ok(())
}

#[derive(Accounts)]
pub struct ChangeAssetConfigInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.get_asset_address().as_ref()],
        bump = asset.get_account_bump()
    )]
    pub asset: Account<'info, Asset>,

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

pub fn change_asset_config_handler(
    ctx: Context<ChangeAssetConfigInstructionAccounts>,
) -> Result<()> {
    let asset = &mut ctx.accounts.asset;
    let proposal = &ctx.accounts.proposal;

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *proposal.get_proposer(),
        TokenError::InvalidProposer
    );

    // Validate proposal
    require!(
        proposal.get_state() == ProposalState::Passed,
        TokenError::ProposalNotPassed
    );

    match proposal.get_config_change() {
        ConfigChange::ChangeAssetConfig {
            asset: asset_key,
            config_type,
        } => {
            require!(
                *asset_key == *asset.get_asset_address(),
                TokenError::InvalidAsset
            );

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
            }
        }
        _ => return err!(TokenError::InvalidConfigChange),
    }

    Ok(())
}

use crate::ProposalTarget;
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
#[inline(always)] /// This function is only called once in the handler
fn change_group_config_checks(
    ctx: &Context<ChangeGroupConfigInstructionAccounts>,
)->Result<()>{
    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *ctx.accounts.proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Validate proposal
    
    let now = Clock::get()?.unix_timestamp;

    require_gt!(now, 
        ctx.accounts.proposal.get_valid_from_timestamp()?, 
        MultisigError::ProposalStillTimelocked
    );

    require!(
        ctx.accounts.proposal.get_state() == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    require_gte!(
        ctx.accounts.proposal.get_proposal_index(),
        ctx.accounts.group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Updates group-wide configuration (e.g, timelock, thresholds, expiry), 
/// it must be triggered by an approved proposal and can then be called by anyone
pub fn change_group_config_handler(
    ctx: Context<ChangeGroupConfigInstructionAccounts>,
) -> Result<()> {
    change_group_config_checks(&ctx)?;
    
    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    group.update_stale_proposal_index();


    match proposal.get_config_change() {
        // Change the group configuration
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
            _ => return Err(MultisigError::UnexpectedConfigChange.into()),
        },
        _ => return Err(MultisigError::InvalidConfigChange.into()),
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

#[inline(always)] /// This function is only called once in the handler
fn change_asset_config_checks(
    ctx: &Context<ChangeAssetConfigInstructionAccounts>    
)->Result<()>{

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *ctx.accounts.proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Validate proposal
    
    let now = Clock::get()?.unix_timestamp;

    require_gt!(now, 
        ctx.accounts.proposal.get_valid_from_timestamp()?, 
        MultisigError::ProposalStillTimelocked
    );

    require!(
        ctx.accounts.proposal.get_state() == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    require_gte!(
        ctx.accounts.proposal.get_proposal_index(),
        ctx.accounts.group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Updates asset-wide configuration (e.g, timelock, thresholds, expiry), 
/// it must be triggered by an approved proposal and can then be called by anyone
pub fn change_asset_config_handler(
    ctx: Context<ChangeAssetConfigInstructionAccounts>,
) -> Result<()> {

    change_asset_config_checks(&ctx)?;

    let group = &mut ctx.accounts.group;
    let asset = &mut ctx.accounts.asset;
    let proposal = &ctx.accounts.proposal;
    
    group.update_stale_proposal_index();
    

    match proposal.get_config_change() {
        // Change the asset configuration
        ConfigChange::ChangeAssetConfig {
            config_type,
        } => {

            let asset_key = match proposal.get_target() {
                &ProposalTarget::Asset(asset_key) => asset_key,
                ProposalTarget::Group => return Err(MultisigError::InvalidConfigChange.into()),
            };

            require!(
                asset_key == *asset.get_asset_address(),
                MultisigError::InvalidAsset
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
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

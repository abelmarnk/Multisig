use anchor_lang::prelude::*;

use crate::{
    state::{
        asset::Asset,
        group::Group,
        member::AssetMember,
        proposal::{ConfigChange, ConfigProposal, ProposalState},
        MultisigError,
    },
    GroupMember,
};

#[derive(Accounts)]
pub struct RemoveGroupMemberInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"member", group.key().as_ref(), group_member_account.key().as_ref()],
        bump = group_member_account.get_account_bump(),
        close = rent_collector
    )]
    pub group_member_account: Account<'info, GroupMember>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer
    )]
    pub proposal: Account<'info, ConfigProposal>,

    #[account(mut)]
    /// CHECK: Rent collector
    pub rent_collector: UncheckedAccount<'info>,

    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]// This function is only called once in the handler
// Other checks are performed in the handler
fn remove_group_member_checks(
    ctx: &Context<RemoveGroupMemberInstructionAccounts>,
)->Result<()>{

    // Validate rent collector
    require_keys_eq!(
        ctx.accounts.rent_collector.key(),
        *ctx.accounts.group.get_rent_collector(),
        MultisigError::UnexpectedRentCollector
    );

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *ctx.accounts.proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Validate proposal
    require!(
        ctx.accounts.proposal.get_state() == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    let now = Clock::get()?.unix_timestamp;

    require_gt!(
        now, 
        ctx.accounts.proposal.get_valid_from_timestamp()?, 
        MultisigError::ProposalStillTimelocked
    );

    require_gte!(
        ctx.accounts.proposal.get_proposal_index(),
        ctx.accounts.group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Removes an existing group member once a proposal to remove them has passed, 
/// closes their GroupMember account and sends the rent to the rent_collector.
/// it must be triggered by an approved proposal and can then be called by anyone
pub fn remove_group_member_handler(
    ctx: Context<RemoveGroupMemberInstructionAccounts>,
) -> Result<()> {

    // Perform preliminary checks
    remove_group_member_checks(&ctx)?;
    
    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    match proposal.get_config_change() {
        ConfigChange::RemoveGroupMember {
            member: target_member,
        } => {
            require!(
                *target_member == *ctx.accounts.group_member_account.get_user(),
                MultisigError::InvalidMember
            );

            // Decrement group member count
            group.decrement_member_count()?;
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

#[derive(Accounts)]
pub struct RemoveAssetMemberInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// Asset being governed
    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.get_asset_address().as_ref()],
        bump = asset.get_account_bump()
    )]
    pub asset: Account<'info, Asset>,

    /// We don't add in the group member because though a member is required to be part of the group
    /// to be able to govern an asset, it may still have it's asset membership existing while the group is
    /// removed(in this case since both group and asset membership are checked before any of their actions are 
    /// authorized and their asset membership is useless as it should be), this because of the fact 
    /// that because of solana's transaction size limits not all asset memberships would be able to be removed at once.
    #[account(
        mut,
        close = rent_collector,
        seeds = [b"asset-member", group.key().as_ref(), 
            asset.get_asset_address().as_ref(), asset_member_account.key().as_ref()],
        bump = asset_member_account.get_account_bump()
    )]
    pub asset_member_account: Account<'info, AssetMember>,

    /// ConfigProposal approving this removal
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer
    )]
    pub proposal: Account<'info, ConfigProposal>,

    #[account(mut)]
    /// CHECK: rent collector of the group
    pub rent_collector: UncheckedAccount<'info>,

    /// Account that opened the proposal, receives proposal's rent
    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]// This function is only called once in the handler
// Other checks are performed in the handler
fn remove_asset_member_checks(
    ctx: &Context<RemoveAssetMemberInstructionAccounts>,
)->Result<()>{

    // Validate rent collector
    require_keys_eq!(
        ctx.accounts.rent_collector.key(),
        *ctx.accounts.group.get_rent_collector(),
        MultisigError::UnexpectedRentCollector
    );

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *ctx.accounts.proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Validate proposal
    require!(
        ctx.accounts.proposal.get_state() == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    let now = Clock::get()?.unix_timestamp;

    require_gt!(
        now, 
        ctx.accounts.proposal.get_valid_from_timestamp()?, 
        MultisigError::ProposalStillTimelocked
    );

    require_gte!(
        ctx.accounts.proposal.get_proposal_index(),
        ctx.accounts.group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Removes an existing asset member once a proposal to remove them has passed,
/// closes their AssetMember account and sends the rent to the rent_collector.
/// It is not checked that they have a corresponding group account since one(AssetMember) could
/// exist without the other(GroupMember)
pub fn remove_asset_member_handler(
    ctx: Context<RemoveAssetMemberInstructionAccounts>,
) -> Result<()> {

    // Perform preliminary checks
    remove_asset_member_checks(&ctx)?;

    let asset = &mut ctx.accounts.asset;
    let proposal = &ctx.accounts.proposal;
    let asset_member = &ctx.accounts.asset_member_account;

    match proposal.get_config_change() {
        ConfigChange::RemoveAssetMember {
            member: target_member,
            asset_address,
        } => {
            // Asset address must match the provided asset account
            require_keys_eq!(
                *asset_address,
                *asset.get_asset_address(),
                MultisigError::InvalidAsset
            );

            // Validate the asset_member PDA points to expected member and asset
            require_keys_eq!(
                *asset_member.get_asset(),
                *asset_address,
                MultisigError::InvalidAsset
            );

            require_keys_eq!(
                *asset_member.get_user(),
                *target_member,
                MultisigError::InvalidMember
            );

            asset.decrement_member_count()?;
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

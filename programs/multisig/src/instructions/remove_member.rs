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
    /// Group
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

    /// ConfigProposal approving this removal
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer
    )]
    pub proposal: Account<'info, ConfigProposal>,

    #[account(mut)]
    /// CHECK: Collector of the `group_member_account` rent
    pub rent_collector: UncheckedAccount<'info>,

    /// Account that opened the proposal, receives proposal's rent
    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn remove_group_member_handler(
    ctx: Context<RemoveGroupMemberInstructionAccounts>,
) -> Result<()> {
    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Validate proposal
    require!(
        proposal.get_state() == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    require_gte!(
        proposal.get_proposal_index(),
        group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    group.set_proposal_index_after_stale(
        proposal
            .get_proposal_index()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    );

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
    /// Group
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

    // We don't add in the group member because though a member is required to be part of the group
    // to be able to govern an asset, it may still it's asset membership existing while the group is
    // lost(in this case since both group and asset membership are checked before any action is 
    // authorized their asset membership is useless as it should be), this because of the fact 
    // that because of solana's transaction size limits not all would be able to be removed at once.
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
    /// CHECK: Collector of the `group_member_account` rent
    pub rent_collector: UncheckedAccount<'info>,

    /// Account that opened the proposal, receives proposal's rent
    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn remove_asset_member_handler(
    ctx: Context<RemoveAssetMemberInstructionAccounts>,
) -> Result<()> {
    let group = &mut ctx.accounts.group;
    let asset = &mut ctx.accounts.asset;
    let proposal = &ctx.accounts.proposal;
    let asset_member = &ctx.accounts.asset_member_account;

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Validate proposal state
    require!(
        proposal.get_state() == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    // Ensure proposal is not stale
    require_gte!(
        proposal.get_proposal_index(),
        group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    // Advance group's stale index
    group.set_proposal_index_after_stale(
        proposal
            .get_proposal_index()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    );

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

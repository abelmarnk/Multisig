use anchor_lang::prelude::*;

use crate::{
    state::{
        asset::Asset,
        group::Group,
        member::AssetMember,
        proposal::{ConfigChange, ConfigProposal, ProposalState},
        TokenError,
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

    /// Collector of the `group_member_account` rent
    #[account(mut)]
    pub rent_collector: Signer<'info>,

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
        ConfigChange::RemoveGroupMember {
            member: target_member,
        } => {
            require!(
                *target_member == *ctx.accounts.group_member_account.get_user(),
                TokenError::InvalidMember
            );

            // Decrement group member count
            group.decrement_member_count()?;
        }
        _ => return Err(TokenError::InvalidConfigChange.into()),
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

    /// Collector of the `asset_member_account` rent
    #[account(mut)]
    pub rent_collector: Signer<'info>,

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
        TokenError::InvalidProposer
    );

    // Validate proposal state
    require!(
        proposal.get_state() == ProposalState::Passed,
        TokenError::ProposalNotPassed
    );

    // Ensure proposal is not stale
    require_gte!(
        proposal.get_proposal_index(),
        group.get_proposal_index_after_stale(),
        TokenError::ProposalStale
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
                TokenError::InvalidAsset
            );

            // Validate the asset_member PDA points to expected member and asset
            require_keys_eq!(
                asset_member.get_asset(),
                *asset_address,
                TokenError::InvalidAsset
            );
            require_keys_eq!(
                asset_member.get_user(),
                *target_member,
                TokenError::InvalidMember
            );

            asset.decrement_member_count();
        }
        _ => return Err(TokenError::InvalidConfigChange.into()),
    }

    Ok(())
}

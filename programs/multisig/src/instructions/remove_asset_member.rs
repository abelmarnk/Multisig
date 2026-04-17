use anchor_lang::prelude::*;

use crate::state::{
    asset::Asset,
    group::Group,
    member::AssetMember,
    proposal::{ConfigChange, ConfigProposal, ProposalState},
    MultisigError,
};

#[derive(Accounts)]
pub struct RemoveAssetMemberInstructionAccounts<'info> {
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

    /// An asset member may outlive group membership thus the group member is not required.
    #[account(
        mut,
        close = rent_collector,
        seeds = [b"asset-member", group.key().as_ref(), 
            asset.asset_address.as_ref(), asset_member_account.user.as_ref()],
        bump = asset_member_account.account_bump
    )]
    pub asset_member_account: Account<'info, AssetMember>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer
    )]
    pub proposal: Account<'info, ConfigProposal>,

    /// CHECK: Validated against group.rent_collector in checks().
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,

    /// CHECK: Must match the proposer stored in the proposal; receives closed-account rent.
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(ctx: &Context<RemoveAssetMemberInstructionAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require_keys_eq!(
        ctx.accounts.rent_collector.key(),
        ctx.accounts.group.rent_collector,
        MultisigError::UnexpectedRentCollector
    );

    require_keys_eq!(
        ctx.accounts.proposer.key(),
        ctx.accounts.proposal.proposer,
        MultisigError::InvalidProposer
    );

    // Validate proposal
    require!(
        ctx.accounts.proposal.state == ProposalState::Passed,
        MultisigError::ProposalNotPassed
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

    require_gte!(
        ctx.accounts.proposal.proposal_index,
        ctx.accounts.group.proposal_index_after_stale,
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Removes an existing asset member once a proposal to remove them has passed,
/// closes their AssetMember account and sends the rent to the rent_collector.
/// It is not checked that they have a corresponding group account since one(AssetMember) could
/// exist without the other(GroupMember).
pub fn remove_asset_member_handler(
    ctx: Context<RemoveAssetMemberInstructionAccounts>,
) -> Result<()> {
    checks(&ctx)?;

    let asset = &mut ctx.accounts.asset;
    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;
    let asset_member = &ctx.accounts.asset_member_account;

    match &proposal.config_change {
        ConfigChange::RemoveAssetMember {
            member: target_member,
            asset_address,
        } => {
            require_keys_eq!(
                *asset_address,
                asset.asset_address,
                MultisigError::InvalidAsset
            );

            require_keys_eq!(
                asset_member.asset,
                *asset_address,
                MultisigError::InvalidAsset
            );

            require_keys_eq!(
                asset_member.user,
                *target_member,
                MultisigError::InvalidMember
            );

            asset.decrement_member_count()?;
            group.update_stale_proposal_index();
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

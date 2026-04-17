use crate::state::error::*;
use crate::state::{
    asset::Asset,
    group::Group,
    member::{AssetMember, GroupMember},
    proposal::{ConfigChange, ConfigProposal, ProposalState, ProposalTarget},
};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddAssetMemberInstructionArgs {
    pub new_member: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: AddAssetMemberInstructionArgs)]
pub struct AddAssetMemberInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    // Asset being governed
    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.asset_address.as_ref()],
        bump = asset.account_bump
    )]
    pub asset: Account<'info, Asset>,

    // ConfigProposal approving this addition
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    // Account that opened the proposal, receives rent
    /// CHECK: Must match the proposer stored in the proposal; receives closed-account rent.
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,

    // Corresponding group member must exist
    #[account(
        seeds = [b"member", group.key().as_ref(), args.new_member.as_ref()],
        bump = group_member.account_bump
    )]
    pub group_member: Account<'info, GroupMember>,

    // AssetMember PDA for new asset member
    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset-member", group.key().as_ref(), asset.asset_address.as_ref(), args.new_member.as_ref()],
        bump
    )]
    pub new_asset_member: Account<'info, AssetMember>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(ctx: &Context<AddAssetMemberInstructionAccounts>) -> Result<()> {
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

/// Adds a pre-existing group member to govern an existing asset, storing their key and weight
/// and permissions, as well as the group key and asset key for indexing.
/// it must be triggered by an approved proposal and can then be called by anyone.
pub fn add_asset_member_handler(
    ctx: Context<AddAssetMemberInstructionAccounts>,
    args: AddAssetMemberInstructionArgs,
) -> Result<()> {
    checks(&ctx)?;

    let AddAssetMemberInstructionArgs { new_member } = args;

    let group = &mut ctx.accounts.group;
    let asset = &mut ctx.accounts.asset;
    let proposal = &ctx.accounts.proposal;

    group.update_stale_proposal_index();

    match &proposal.target {
        ProposalTarget::Asset(asset_address) => {
            require_keys_eq!(
                *asset_address,
                asset.asset_address,
                MultisigError::InvalidAsset
            );
        }
        ProposalTarget::Group => return Err(MultisigError::InvalidConfigChange.into()),
    }

    match &proposal.config_change {
        ConfigChange::AddAssetMember {
            weight,
            member,
            permissions,
            asset_address,
        } => {
            require_keys_eq!(
                *asset_address,
                asset.asset_address,
                MultisigError::InvalidAsset
            );
            require_keys_eq!(*member, new_member, MultisigError::InvalidMember);

            ctx.accounts.new_asset_member.set_inner(AssetMember::new(
                new_member,
                group.key(),
                asset.asset_address,
                *permissions,
                *weight,
                ctx.bumps.new_asset_member,
                group.max_member_weight,
            )?);

            asset.increment_member_count()?;
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

use crate::state::error::*;
use crate::state::{
    asset::Asset,
    group::Group,
    member::{AssetMember, GroupMember, Permissions},
    proposal::{ConfigChange, ConfigProposal, ProposalState},
};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddGroupMemberInstructionArgs {
    pub new_member: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: AddGroupMemberInstructionArgs)]
pub struct AddGroupMemberInstructionAccounts<'info> {
    /// Governance group
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// ConfigProposal approving this addition
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    /// Account that opened the proposal, receives rent
    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    /// The new group member account to be initialized
    #[account(
        init,
        space = 8 + GroupMember::INIT_SPACE,
        payer = payer,
        seeds = [b"member", group.key().as_ref(), args.new_member.as_ref()],
        bump
    )]
    pub new_group_member: Account<'info, GroupMember>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn add_group_member_handler(
    ctx: Context<AddGroupMemberInstructionAccounts>,
    args: AddGroupMemberInstructionArgs, // We pass in the argument because of the seed checks above
                                         // A helper function could be added later to extract it from the proposal
) -> Result<()> {
    let AddGroupMemberInstructionArgs { new_member } = args;

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

    // Add member
    match proposal.get_config_change() {
        ConfigChange::AddGroupMember {
            member,
            weight,
            permissions,
        } => {
            require!(*member == new_member, MultisigError::InvalidMember);

            ctx.accounts.new_group_member.set_inner(GroupMember::new(
                new_member,
                group.key(),
                Permissions::new(*permissions)?,
                *weight,
                ctx.bumps.new_group_member,
                group.get_max_member_weight(),
            )?);

            group.increment_member_count()?;
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddAssetMemberInstructionArgs {
    pub new_member: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: AddAssetMemberInstructionArgs)]
pub struct AddAssetMemberInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// Asset being governed
    #[account(mut)]
    pub asset: Account<'info, Asset>,

    /// ConfigProposal approving this addition
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    /// Account that opened the proposal, receives rent
    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    /// Corresponding group member must exist
    #[account(
        seeds = [b"member", group.key().as_ref(), args.new_member.as_ref()],
        bump = group_member.get_account_bump()
    )]
    pub group_member: Account<'info, GroupMember>,

    /// AssetMember PDA for new asset member
    #[account(
        init,
        payer = payer,
        space = 8 + AssetMember::INIT_SPACE,
        seeds = [b"asset_member", group.key().as_ref(), asset.get_asset_address().as_ref(), args.new_member.as_ref()],
        bump
    )]
    pub new_asset_member: Account<'info, AssetMember>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn add_asset_member_handler(
    ctx: Context<AddAssetMemberInstructionAccounts>,
    args: AddAssetMemberInstructionArgs,
) -> Result<()> {
    let AddAssetMemberInstructionArgs { new_member } = args;

    let group = &mut ctx.accounts.group;
    let asset = &mut ctx.accounts.asset;
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
        ConfigChange::AddAssetMember {
            weight,
            member,
            permissions,
            asset_address,
        } => {
            require!(
                *asset_address == *asset.get_asset_address(),
                MultisigError::InvalidAsset
            );
            require!(*member == new_member, MultisigError::InvalidMember);

            ctx.accounts.new_asset_member.set_inner(AssetMember::new(
                new_member,
                group.key(),
                *asset.get_asset_address(),
                Permissions::new(*permissions)?,
                *weight,
                ctx.bumps.new_asset_member,
                group.get_max_member_weight(),
            )?);

            asset.increment_member_count()?;
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

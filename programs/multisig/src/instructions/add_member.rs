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
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    // ConfigProposal approving this addition
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    // Account that opened the proposal, receives rent
    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    // The new group member account to be initialized
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
#[inline(always)] /// This function is only called once in the handler
fn add_group_member_checks(
    ctx: &Context<AddGroupMemberInstructionAccounts>
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

/// Adds a new member to an existing group, storing their key and weight and permissions,
/// as well as the group key for indexing.
/// it must be triggered by an approved proposal and can then be called by anyone
pub fn add_group_member_handler(
    ctx: Context<AddGroupMemberInstructionAccounts>,
    args: AddGroupMemberInstructionArgs, // We pass in the argument because of the seed checks above
                                         // A helper function could be added later to extract it from the proposal
) -> Result<()> {
    let AddGroupMemberInstructionArgs { new_member } = args;

    // Perform preliminary checks
    add_group_member_checks(&ctx)?;

    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;
   
    group.update_stale_proposal_index();


    // Add member
    match proposal.get_config_change() {
        // Add in the new group member        
        ConfigChange::AddGroupMember {
            member,
            weight,
            permissions,
        } => {
            require!(*member == new_member, MultisigError::InvalidMember);

            ctx.accounts.new_group_member.set_inner(GroupMember::new(
                new_member,
                group.key(),
                *permissions,
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

    // Asset being governed
    #[account(mut)]
    pub asset: Account<'info, Asset>,

    // ConfigProposal approving this addition
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    // Account that opened the proposal, receives rent
    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    // Corresponding group member must exist
    #[account(
        seeds = [b"member", group.key().as_ref(), args.new_member.as_ref()],
        bump = group_member.get_account_bump()
    )]
    pub group_member: Account<'info, GroupMember>,

    // AssetMember PDA for new asset member
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

#[inline(always)] /// This function is only called once in the handler
fn add_asset_member_checks(
    ctx: &Context<AddAssetMemberInstructionAccounts>,
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
/// Adds a pre-existing group member to govern an existing asset, storing their key and weight
///  and permissions, as well as the group key and asset key for indexing.
/// it must be triggered by an approved proposal and can then be called by anyone
pub fn add_asset_member_handler(
    ctx: Context<AddAssetMemberInstructionAccounts>,
    args: AddAssetMemberInstructionArgs,// We pass in the argument because of the seed checks above
                                        // A helper function could be added later to extract it from the proposal

) -> Result<()> {

    // Perform preliminary checks
    add_asset_member_checks(&ctx)?;

    let AddAssetMemberInstructionArgs { new_member } = args;

    let group = &mut ctx.accounts.group;
    let asset = &mut ctx.accounts.asset;
    let proposal = &ctx.accounts.proposal;

    group.update_stale_proposal_index();


    match proposal.get_config_change() {
        // Add in the new asset member
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
                *permissions,
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

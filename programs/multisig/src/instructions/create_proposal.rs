use crate::state::*;
use anchor_lang::{prelude::*, solana_program::hash::HASH_BYTES as HASH_BYTES_LENGTH};

// Instruction arguments struct for CreateNormalProposalInstructionAccounts
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateNormalProposalInstructionArgs {
    pub proposal_seed: Pubkey,
    pub asset_keys: Vec<Pubkey>,
    pub asset_indices: Vec<u8>,
    pub authority_bumps: Vec<u8>,
    pub expiry_offset: Option<u32>,
    pub timelock_offset: Option<u32>,
    pub instruction_hash: [u8; HASH_BYTES_LENGTH],
}

// Instruction arguments struct for CreateConfigProposal
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateConfigProposalInstructionArgs {
    pub proposal_seed: Pubkey,
    pub timelock_offset: Option<u32>,
    pub expiry_offset: Option<u32>,
    pub config_change: ConfigChange,
}

#[derive(Accounts)]
#[instruction(args: CreateNormalProposalInstructionArgs)]
pub struct CreateNormalProposalInstructionAccounts<'info> {
    /// Group being governed
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// The proposer (must be group member)
    #[account(mut)]
    pub proposer: Signer<'info>,

    /// Proof proposer is a member
    #[account(
        seeds = [b"member", group.key().as_ref(), proposer.key.as_ref()],
        bump = proposer_group_account.get_account_bump()
    )]
    pub proposer_group_account: Account<'info, GroupMember>,

    /// The new proposal account
    #[account(
        init,
        payer = proposer,
        space = 8 + NormalProposal::get_size(args.asset_keys.len()),
        seeds = [b"proposal", group.key().as_ref(), args.proposal_seed.as_ref()],
        bump,
    )]
    pub proposal: Account<'info, NormalProposal>,

    pub system_program: Program<'info, System>,
}

pub fn create_normal_proposal_handler(
    ctx: Context<CreateNormalProposalInstructionAccounts>,
    args: CreateNormalProposalInstructionArgs,
) -> Result<()> {

    let CreateNormalProposalInstructionArgs {
        proposal_seed,
        asset_keys,
        asset_indices,
        authority_bumps,
        expiry_offset,
        timelock_offset,
        instruction_hash,
    } = args;

    // Verify proposer owns membership asset
    require!(
        ctx.accounts.proposer_group_account.has_propose(),
        MultisigError::InsufficientPermissions
    );

    // Verify asset count constraint
    require_gte!(
        constants::MAX_ASSET_USE,
        asset_keys.len(),
        MultisigError::TooManyAssets
    );

    require_eq!(
        asset_keys.len(),
        asset_indices.len(),
        MultisigError::LengthMismatch
    );

    require_eq!(
        asset_indices.len(),
        authority_bumps.len(),
        MultisigError::LengthMismatch
    );

    // Ensure sorted + no duplicates
    for i in 1..asset_keys.len() {
        require!(
            asset_keys[i - 1] < asset_keys[i],
            MultisigError::AssetsNotSortedOrDuplicate
        );
    }

    // Construct ProposalAssets
    let proposal_assets: Vec<ProposalAsset> = asset_keys
        .into_iter()
        .zip(asset_indices.into_iter())
        .zip(authority_bumps.into_iter())
        .map(|((key, index), bump)| ProposalAsset::new(index, bump, key))
        .collect();

    let proposal = &mut ctx.accounts.proposal;
    let proposer = &mut ctx.accounts.proposer;
    let group = &mut ctx.accounts.group;

    proposal.set_inner(NormalProposal::new(
        *proposer.key,
        proposal_seed,
        group.key(),
        proposal_assets,
        ctx.bumps.proposal,
        group.get_and_increment_proposal_index()?,
        instruction_hash,
        timelock_offset.unwrap_or(group.get_timelock_offset()),
        expiry_offset.unwrap_or(group.get_expiry_offset()),
    )?);

    Ok(())
}

#[derive(Accounts)]
#[instruction(args: CreateConfigProposalInstructionArgs)]
pub struct CreateConfigProposalInstructionAccounts<'info> {
    /// The proposer (must be a group member)
    #[account(mut)]
    pub proposer: Signer<'info>,

    /// Group being governed
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// Optional: Asset being governed (required for asset-targeted config changes)
    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.get_asset_address().as_ref()],
        bump = asset.get_account_bump()
    )]
    pub asset: Option<Account<'info, Asset>>,

    /// Proof proposer is a group member
    #[account(
        seeds = [b"member", group.key().as_ref(), proposer.key().as_ref()],
        bump = proposer_group_account.get_account_bump()
    )]
    pub proposer_group_account: Account<'info, GroupMember>,

    /// The new proposal account (PDA)
    #[account(
        init,
        payer = proposer,
        space = 8 + ConfigProposal::INIT_SPACE,
        seeds = [b"proposal", group.key().as_ref(), args.proposal_seed.as_ref()],
        bump,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    pub system_program: Program<'info, System>,
}

pub fn create_config_proposal_handler(
    ctx: Context<CreateConfigProposalInstructionAccounts>,
    args: CreateConfigProposalInstructionArgs,
) -> Result<()> {

    let CreateConfigProposalInstructionArgs {
        proposal_seed,
        timelock_offset,
        expiry_offset,
        config_change,
    } = args;


    let proposer_key = ctx.accounts.proposer.key();
    let group = &mut ctx.accounts.group;
    let proposer_member = &ctx.accounts.proposer_group_account;
    let proposal = &mut ctx.accounts.proposal;

    // Proposer must have propose permission
    require!(
        proposer_member.has_propose(),
        MultisigError::InsufficientPermissions
    );

    // Use provided offsets or group defaults
    let expiry = expiry_offset.unwrap_or(group.get_expiry_offset());

    let timelock = timelock_offset.unwrap_or(group.get_timelock_offset());

    // If this is a group-targeted change, construct a group ConfigProposal
    if config_change.is_group_change() {
        let new_proposal = ConfigProposal::new(
            proposer_key,
            proposal_seed,
            group.key(),
            ctx.bumps.proposal,
            group.get_and_increment_proposal_index()?,
            timelock,
            expiry,
            ProposalTarget::Group,
            config_change,
        )?;

        proposal.set_inner(new_proposal);
        return Ok(());
    } else {
        // Asset proposal
        let asset = ctx
            .accounts
            .asset
            .as_ref()
            .ok_or(MultisigError::AssetNotProvided)?;

        proposal.set_inner(ConfigProposal::new(
            proposer_key,
            proposal_seed,
            group.key(),
            ctx.bumps.proposal,
            group.get_and_increment_proposal_index()?,
            timelock,
            expiry,
            ProposalTarget::Asset(*asset.get_asset_address()),
            config_change,
        )?);
    };

    Ok(())
}

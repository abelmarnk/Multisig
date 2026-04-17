use crate::state::*;
use anchor_lang::{prelude::*, solana_program::hash::HASH_BYTES as HASH_BYTES_LENGTH};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateNormalProposalInstructionArgs {
    pub proposal_seed: Pubkey,
    pub asset_keys: Vec<Pubkey>,
    pub asset_indices: Vec<AssetIndex>,
    pub authority_bumps: Vec<u8>,
    pub timelock_offset: u32,
    pub proposal_deadline_timestamp: i64,
    /// Hashes of each instruction in the proposal transaction, in order.
    pub instruction_hashes: Vec<[u8; HASH_BYTES_LENGTH]>,
}

#[derive(Accounts)]
#[instruction(args: CreateNormalProposalInstructionArgs)]
pub struct CreateNormalProposalInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(mut)]
    pub proposer: Signer<'info>,

    #[account(
        seeds = [b"member", group.key().as_ref(), proposer.key.as_ref()],
        bump = proposer_group_account.account_bump
    )]
    pub proposer_group_account: Account<'info, GroupMember>,

    #[account(
        init,
        payer = proposer,
        space = 8 + NormalProposal::get_size(args.asset_keys.len(), args.instruction_hashes.len()),
        seeds = [b"proposal", group.key().as_ref(), args.proposal_seed.as_ref()],
        bump,
    )]
    pub proposal: Account<'info, NormalProposal>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(
    ctx: &Context<CreateNormalProposalInstructionAccounts>,
    args: &CreateNormalProposalInstructionArgs,
) -> Result<()> {
    require!(
        ctx.accounts.proposer_group_account.has_propose(),
        MultisigError::InsufficientPermissions
    );

    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require!(!args.asset_keys.is_empty(), MultisigError::AssetNotProvided);

    require_gte!(
        constants::MAX_ASSET_USE,
        args.asset_keys.len(),
        MultisigError::TooManyAssets
    );

    require_eq!(
        args.asset_keys.len(),
        args.asset_indices.len(),
        MultisigError::LengthMismatch
    );

    require_eq!(
        args.asset_indices.len(),
        args.authority_bumps.len(),
        MultisigError::LengthMismatch
    );

    for i in 1..args.asset_keys.len() {
        require!(
            args.asset_keys[i - 1] < args.asset_keys[i],
            MultisigError::AssetsNotSortedOrDuplicate
        );
    }

    for i in 1..args.asset_indices.len() {
        require!(
            args.asset_indices[i - 1] < args.asset_indices[i],
            MultisigError::InvalidAssetIndex
        );
    }

    require_gt!(
        args.proposal_deadline_timestamp,
        Clock::get()?.unix_timestamp,
        MultisigError::ProposalExpired
    );

    require_gte!(
        args.timelock_offset,
        ctx.accounts.group.minimum_timelock,
        MultisigError::TimelockBelowMinimum
    );

    require!(
        !args.instruction_hashes.is_empty(),
        MultisigError::EmptyInstructions
    );

    Ok(())
}

/// Creates a normal proposal. Requires Propose permission.
pub fn create_normal_proposal_handler(
    ctx: Context<CreateNormalProposalInstructionAccounts>,
    args: CreateNormalProposalInstructionArgs,
) -> Result<()> {
    checks(&ctx, &args)?;

    let CreateNormalProposalInstructionArgs {
        proposal_seed,
        asset_keys,
        asset_indices,
        authority_bumps,
        timelock_offset,
        instruction_hashes,
        proposal_deadline_timestamp,
    } = args;

    let proposal_assets: Vec<ProposalAsset> = asset_keys
        .into_iter()
        .zip(asset_indices)
        .zip(authority_bumps)
        .map(|((key, idx), bump)| {
            ProposalAsset::new(idx.instruction_index, idx.account_index, bump, key)
        })
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
        proposal_deadline_timestamp,
        instruction_hashes,
        timelock_offset,
    )?);

    Ok(())
}

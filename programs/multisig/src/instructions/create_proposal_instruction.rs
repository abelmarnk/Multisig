use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash;

use crate::{
    state::{
        error::MultisigError, Asset, AssetIndex, Group, NormalProposal, ProposalTransaction,
        SerializableInstruction,
    },
    ProposalState,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateProposalTransactionInstructionArgs {
    /// Serialized bytes for each instruction, in order.
    pub raw_instructions: Vec<Vec<u8>>,
}

impl CreateProposalTransactionInstructionArgs {
    /// Total serialized size of all raw instructions, including the outer Vec length prefix.
    pub fn instructions_total_size(&self) -> usize {
        4 + self
            .raw_instructions
            .iter()
            .map(|r| {
                // Each SerializableInstruction on-chain: program_id(32) + accounts vec(4 + n*34) + data vec(4 + m)
                // We use the raw bytes length as a conservative upper bound since we'll
                // deserialize and re-serialize; the actual sizes match because borsh is used both ways.
                r.len()
            })
            .sum::<usize>()
    }
}

#[derive(Accounts)]
#[instruction(args: CreateProposalTransactionInstructionArgs)]
pub struct CreateProposalTransactionInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
    )]
    pub proposal: Account<'info, NormalProposal>,

    #[account(
        init,
        payer = payer,
        space = 8 + ProposalTransaction::get_size(proposal.assets.len(), args.instructions_total_size()),
        seeds = [b"proposal-transaction", proposal.key().as_ref()],
        bump
    )]
    pub proposal_transaction: Account<'info, ProposalTransaction>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(
    ctx: &Context<CreateProposalTransactionInstructionAccounts>,
    args: &CreateProposalTransactionInstructionArgs,
) -> Result<Vec<SerializableInstruction>> {
    require!(
        ctx.accounts.proposal.state == ProposalState::Open,
        MultisigError::ProposalNotOpen
    );

    require_keys_eq!(
        ctx.accounts.proposal.group,
        ctx.accounts.group.key(),
        MultisigError::UnexpectedGroup
    );

    require_gte!(
        ctx.accounts.proposal.proposal_index,
        ctx.accounts.group.proposal_index_after_stale,
        MultisigError::ProposalStale
    );

    require_gt!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        Clock::get()?.unix_timestamp,
        MultisigError::ProposalExpired
    );

    require!(
        !args.raw_instructions.is_empty(),
        MultisigError::EmptyInstructions
    );

    require_eq!(
        args.raw_instructions.len(),
        ctx.accounts.proposal.instruction_hashes.len(),
        MultisigError::LengthMismatch
    );

    let proposal_assets = &ctx.accounts.proposal.assets;
    require_eq!(
        ctx.remaining_accounts.len(),
        proposal_assets.len(),
        MultisigError::LengthMismatch
    );

    let group_key = ctx.accounts.group.key();
    for (asset_info, proposal_asset) in ctx.remaining_accounts.iter().zip(proposal_assets.iter()) {
        require_keys_eq!(*asset_info.owner, crate::ID, MultisigError::InvalidAsset);
        let asset_data = asset_info.try_borrow_data()?;
        let asset_account = Asset::try_deserialize(&mut &asset_data[..])
            .map_err(|_| MultisigError::InvalidAsset)?;
        require_keys_eq!(
            asset_account.asset_address,
            proposal_asset.asset,
            MultisigError::InvalidAsset
        );

        let (expected_asset, _) = Pubkey::find_program_address(
            &[b"asset", group_key.as_ref(), proposal_asset.asset.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            expected_asset,
            asset_info.key(),
            MultisigError::InvalidAsset
        );

        let (_, expected_authority_bump) = Pubkey::find_program_address(
            &[
                b"authority",
                group_key.as_ref(),
                proposal_asset.asset.as_ref(),
            ],
            &crate::ID,
        );
        require_eq!(
            asset_account.authority_bump,
            expected_authority_bump,
            MultisigError::InvalidAsset
        );
        require_eq!(
            proposal_asset.authority_bump,
            expected_authority_bump,
            MultisigError::InvalidAsset
        );
    }

    let mut serializable_instructions = Vec::with_capacity(args.raw_instructions.len());

    // Hash check - each raw instruction must match its stored hash to prevent mistakes
    for (raw, expected_hash) in args
        .raw_instructions
        .iter()
        .zip(ctx.accounts.proposal.instruction_hashes.iter())
    {
        let actual_hash = hash::hash(raw).to_bytes();
        require!(
            actual_hash == *expected_hash,
            MultisigError::InvalidInstructionHash
        );

        let ix = SerializableInstruction::try_from_slice(raw)
            .map_err(|_| MultisigError::InstructionDeserializationFailed)?;
        serializable_instructions.push(ix);
    }

    // Verify that each asset key appears at the declared (instruction_index, account_index)
    // position within the submitted instructions. The asset can live in any instruction,
    // not just the first, so no single-instruction constraint is applied.
    for proposal_asset in proposal_assets.iter() {
        let ix = serializable_instructions
            .get(usize::from(proposal_asset.instruction_index))
            .ok_or(MultisigError::InvalidAssetIndex)?;
        let acct_meta = ix
            .accounts
            .get(usize::from(proposal_asset.account_index))
            .ok_or(MultisigError::InvalidAssetIndex)?;
        require_keys_eq!(
            acct_meta.key,
            proposal_asset.asset,
            MultisigError::UnexpectedAsset
        );
    }

    Ok(serializable_instructions)
}

/// Create a transaction associated with a particular proposal.
/// This instruction can be called by anyone.
pub fn create_proposal_transaction_handler(
    ctx: Context<CreateProposalTransactionInstructionAccounts>,
    args: CreateProposalTransactionInstructionArgs,
) -> Result<()> {
    let serializable_instructions = checks(&ctx, &args)?;

    let proposal = &ctx.accounts.proposal;
    let proposal_tx = &mut ctx.accounts.proposal_transaction;

    let proposal_assets = &proposal.assets;

    let asset_indices: Vec<AssetIndex> = proposal_assets
        .iter()
        .map(|a| AssetIndex {
            instruction_index: a.instruction_index,
            account_index: a.account_index,
        })
        .collect();
    let asset_authority_bumps: Vec<[u8; 1]> =
        proposal_assets.iter().map(|a| [a.authority_bump]).collect();

    let transaction = ProposalTransaction::new(
        proposal.key(),
        proposal.group,
        proposal.proposal_index,
        asset_indices,
        asset_authority_bumps,
        serializable_instructions,
        ctx.bumps.proposal_transaction,
    );

    proposal_tx.set_inner(transaction);

    Ok(())
}

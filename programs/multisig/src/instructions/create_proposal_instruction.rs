use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash;

use crate::state::{
    error::MultisigError, Group, NormalProposal, ProposalTransaction, SerializableInstruction,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateProposalTransactionInstructionArgs {
    pub raw_instruction: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(args: CreateProposalTransactionInstructionArgs)]
pub struct CreateProposalTransactionInstructionAccounts<'info> {
    #[account(
        mut,
        close = proposer,
    )]
    pub proposal: Account<'info, NormalProposal>,

    #[account(
        init,
        payer = payer,
        space = 8 + ProposalTransaction::get_size(proposal.get_assets().len(), args.raw_instruction.len()),
        seeds = [b"proposal-transaction", proposal.key().as_ref()],
        bump
    )]
    pub proposal_transaction: Account<'info, ProposalTransaction>,

    #[account(mut)]
    pub group: Account<'info, Group>,

    #[account(mut)]
    pub proposer: SystemAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_proposal_transaction_handler(
    ctx: Context<CreateProposalTransactionInstructionAccounts>,
    args: CreateProposalTransactionInstructionArgs,
) -> Result<()> {

    let CreateProposalTransactionInstructionArgs { raw_instruction } = args;

    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;
    let proposal_tx = &mut ctx.accounts.proposal_transaction;

    // Validate proposer
    require_keys_eq!(
        ctx.accounts.proposer.key(),
        *proposal.get_proposer(),
        MultisigError::InvalidProposer
    );

    // Validate proposal is still valid
    require_gte!(
        proposal.get_proposal_index(),
        group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    // Hash check
    let expected_hash = proposal.get_instruction_hash();
    let actual_hash = hash::hash(&raw_instruction).to_bytes();
    require!(
        actual_hash == *expected_hash,
        MultisigError::InvalidInstructionHash
    );

    // Deserialize into custom struct
    let serializable_instruction: SerializableInstruction =
        SerializableInstruction::try_from_slice(&raw_instruction)
            .map_err(|_| MultisigError::InstructionDeserializationFailed)?;

    // Check proposal assets
    let proposal_assets = proposal.get_assets();
    require_gte!(
        serializable_instruction.accounts.len(),
        proposal_assets.len(),
        MultisigError::NotEnoughAccountKeys
    );

    for proposal_asset in proposal_assets.iter() {
        let index = proposal_asset.get_index() as usize;
        let expected_key = *proposal_asset.get_asset();
        let acct_meta = &serializable_instruction.accounts[index]; // Bounds checked above
        require!(acct_meta.key == expected_key, MultisigError::UnexpectedAsset);
    }

    // Save proposal transaction
    let asset_indices: Vec<u8> = proposal_assets
        .iter()
        .map(|asset| asset.get_index())
        .collect();

    let asset_authority_bumps: Vec<[u8; 1]> = proposal_assets
        .iter()
        .map(|asset| [asset.get_authority_bump()])
        .collect();

    let transaction = ProposalTransaction::new(
        group.key(),
        proposal.get_proposal_index(),
        proposal.get_valid_from_timestamp(),
        asset_indices,
        asset_authority_bumps,
        serializable_instruction,
        ctx.bumps.proposal_transaction,
    );

    proposal_tx.set_inner(transaction);

    Ok(())
}

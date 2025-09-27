use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash;

use crate::{ProposalState, state::{
    Group, NormalProposal, ProposalTransaction, SerializableInstruction, error::MultisigError
}};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateProposalTransactionInstructionArgs {
    pub raw_instruction: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(args: CreateProposalTransactionInstructionArgs)]
pub struct CreateProposalTransactionInstructionAccounts<'info> {

    #[account(
        seeds = [b"proposal", proposal.get_group().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
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
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]// This function is only called once in the handler
// Other checks are performed in the handler
fn create_proposal_transaction_checks(
    ctx: &Context<CreateProposalTransactionInstructionAccounts>,
    args: &CreateProposalTransactionInstructionArgs,
)->Result<()>{
    // Validate proposal
    require!(
        ctx.accounts.proposal.get_state() == ProposalState::Open || 
        ctx.accounts.proposal.get_state() == ProposalState::Passed,
        MultisigError::ProposalNotOpen
    );

    // Hash check
    let expected_hash = ctx.accounts.proposal.get_instruction_hash();
    let actual_hash = hash::hash(&args.raw_instruction).to_bytes();
    require!(
        actual_hash == *expected_hash,
        MultisigError::InvalidInstructionHash
    );

    Ok(())
}

/// Create a transaction associated with a particular proposal
/// This instruction can be called by anyone
pub fn create_proposal_transaction_handler(
    ctx: Context<CreateProposalTransactionInstructionAccounts>,
    args: CreateProposalTransactionInstructionArgs,
) -> Result<()> {

    // Preliminary checks
    create_proposal_transaction_checks(&ctx, &args)?;

    let CreateProposalTransactionInstructionArgs { raw_instruction } = args;

    let proposal = &ctx.accounts.proposal;
    let proposal_tx = &mut ctx.accounts.proposal_transaction;

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
        let acct_meta = 
            &serializable_instruction.accounts.get(index).
            ok_or(MultisigError::InvalidAssetIndex)?;
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
        proposal.key(),
        proposal.get_proposal_index(),
        asset_indices,
        asset_authority_bumps,
        serializable_instruction,
        ctx.bumps.proposal_transaction,
    );

    proposal_tx.set_inner(transaction);

    Ok(())
}

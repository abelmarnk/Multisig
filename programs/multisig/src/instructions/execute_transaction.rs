use crate::state::{error::MultisigError, group::Group, ProposalTransaction};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

#[derive(Accounts)]
pub struct ExecuteProposalTransactionInstructionAccounts<'info> {
    pub group: Account<'info, Group>,

    #[account(mut)]
    pub proposal_transaction: Account<'info, ProposalTransaction>,
}

pub fn execute_proposal_transaction_handler(
    ctx: Context<ExecuteProposalTransactionInstructionAccounts>,
) -> Result<()> {
    let group = &ctx.accounts.group;
    let proposal_transaction = &ctx.accounts.proposal_transaction;
    
    // Ensure the group matches
    require_keys_eq!(proposal_transaction.get_group(), 
        group.key(), MultisigError::UnexpectedGroup
    );

    // Ensure the transaction is ripe(the timelock has expired)
    require_gte!(
        Clock::get()?.unix_timestamp,
        proposal_transaction.get_valid_from(),
        MultisigError::TransactionNotRipe
    );

    // Ensure the proposal for the transaction is not stale.
    require_gte!(
        proposal_transaction.get_proposal_index(),
        group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );
    
    // The actual instruction that was stored previously
    let instruction: Instruction = proposal_transaction.instruction.into_instruction();
    
    // Derive signer seeds for each involved asset
    let mut signer_seeds: Vec<Vec<&[u8]>> =
    Vec::with_capacity(proposal_transaction.asset_indices.len());
    
    let group_key = group.key();

    for asset_index in proposal_transaction.asset_indices.iter() {
        // Find the matching ProposalAsset from the proposal_transaction
        // Bounds were checked during the creation of the instruction
        let asset_key = &proposal_transaction.instruction.accounts[usize::from(*asset_index)].key;

        // PDA seeds: ["authority", group, asset]
        let seeds: Vec<&[u8]> = vec![
            b"authority",
            group_key.as_ref(),
            asset_key.as_ref(),
            &proposal_transaction.asset_authority_bumps[usize::from(*asset_index)],
        ];

        signer_seeds.push(seeds);
    }

    // Collect seeds into slices
    let signer_slices: Vec<&[&[u8]]> = signer_seeds.iter().map(|s| s.as_slice()).collect();

    // Consider building the signer_slices inside the argument space for this call, so we can 
    // avoid building two vectors
    // Execute the stored instruction
    invoke_signed(
        &instruction,
        &ctx.remaining_accounts, // accounts required by the inner instruction
        &signer_slices,          // PDA authority seeds
    )?;

    Ok(())
}

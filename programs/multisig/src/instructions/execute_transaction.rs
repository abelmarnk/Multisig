use crate::{Group, NormalProposal};
use crate::state::{error::MultisigError, ProposalTransaction};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

#[derive(Accounts)]
pub struct ExecuteProposalTransactionInstructionAccounts<'info> {
    #[account(
        seeds = [b"proposal", proposal.get_group().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
    )]
    pub proposal: Account<'info, NormalProposal>,

    #[account(
        mut,
        close = rent_collector,
        seeds = [b"proposal-transaction", proposal.key().as_ref()],
        bump = proposal_transaction.get_account_bump(),
    )]
    pub proposal_transaction: Account<'info, ProposalTransaction>,

    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// CHECK: Rent collector
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>
}

#[inline(always)]// This function is only called once in the handler
fn execute_proposal_transaction_checks(
    ctx: &Context<ExecuteProposalTransactionInstructionAccounts>,
)->Result<()>{
    // Ensure the rent collector matches
    require_keys_eq!(
        *ctx.accounts.group.get_rent_collector(), 
        ctx.accounts.rent_collector.key(), 
        MultisigError::UnexpectedRentCollector
    );

    // Validate proposal
    require!(
        ctx.accounts.proposal.get_state() == crate::ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    let now = Clock::get()?.unix_timestamp;

    require_gt!(
        now,
        ctx.accounts.proposal.get_valid_from_timestamp().map_err(|_| MultisigError::ProposalNotPassed)?,
        MultisigError::ProposalStillTimelocked
    );

    require_gt!(
        ctx.accounts.proposal.get_proposal_index(),
        ctx.accounts.group.get_proposal_index_after_stale(),
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Execute a transaction associated with a particular proposal
/// This instruction can be called by anyone
pub fn execute_proposal_transaction_handler(
    ctx: Context<ExecuteProposalTransactionInstructionAccounts>,
) -> Result<()> {

    execute_proposal_transaction_checks(&ctx)?;

    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;
    let proposal_transaction = &ctx.accounts.proposal_transaction;
    
    group.update_stale_proposal_index();
    
    // The actual instruction that was stored previously
    let instruction: Instruction = proposal_transaction.instruction.into_instruction();
    
    // Derive signer seeds for each involved asset
    let mut signer_seeds: Vec<Vec<&[u8]>> =
    Vec::with_capacity(proposal_transaction.asset_indices.len());
    
    let group_key = proposal.get_group();

    for asset_index in proposal_transaction.asset_indices.iter() {
        // Find the matching ProposalAsset from the proposal_transaction
        // Bounds were checked during the creation of the instruction
        let asset_key = &proposal_transaction.instruction.accounts[usize::from(*asset_index)].key;

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

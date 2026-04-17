use crate::state::{error::MultisigError, ProposalTransaction};
use crate::{Group, NormalProposal};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

#[derive(Accounts)]
pub struct ExecuteProposalTransactionInstructionAccounts<'info> {
    /// Seeds bind proposal to group - proposal.group == group is guaranteed.
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
    )]
    pub proposal: Account<'info, NormalProposal>,

    /// Seeds bind transaction to proposal - proposal_transaction.proposal == proposal is guaranteed.
    #[account(
        mut,
        close = rent_collector,
        seeds = [b"proposal-transaction", proposal.key().as_ref()],
        bump = proposal_transaction.account_bump,
    )]
    pub proposal_transaction: Account<'info, ProposalTransaction>,

    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// CHECK: Rent collector; verified against group.rent_collector in checks().
    #[account(mut)]
    pub rent_collector: UncheckedAccount<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<ExecuteProposalTransactionInstructionAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require_keys_eq!(
        ctx.accounts.group.rent_collector,
        ctx.accounts.rent_collector.key(),
        MultisigError::UnexpectedRentCollector
    );

    // Validate proposal
    require!(
        ctx.accounts.proposal.state == crate::ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    let now = Clock::get()?.unix_timestamp;

    require_gte!(
        now,
        ctx.accounts
            .proposal
            .get_valid_from_timestamp()
            .map_err(|_| MultisigError::ProposalNotPassed)?,
        MultisigError::ProposalStillTimelocked
    );

    require_gte!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        now,
        MultisigError::ProposalExpired
    );

    require_gte!(
        ctx.accounts.proposal.proposal_index,
        ctx.accounts.group.proposal_index_after_stale,
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Executes the transaction attached to a passed normal proposal.
pub fn execute_proposal_transaction_handler(
    ctx: Context<ExecuteProposalTransactionInstructionAccounts>,
) -> Result<()> {
    checks(&ctx)?;

    let proposal = &ctx.accounts.proposal;
    let proposal_transaction = &ctx.accounts.proposal_transaction;
    let group_key = proposal.group;

    // Locate each asset key by following (instruction_index, account_index) into the stored instruction list.
    let mut signer_seeds: Vec<[&[u8]; 4]> =
        Vec::with_capacity(proposal_transaction.asset_indices.len());

    for (position, asset_index) in proposal_transaction.asset_indices.iter().enumerate() {
        let ix = proposal_transaction
            .instructions
            .get(usize::from(asset_index.instruction_index))
            .ok_or(MultisigError::InvalidAssetIndex)?;
        let asset_key = &ix
            .accounts
            .get(usize::from(asset_index.account_index))
            .ok_or(MultisigError::InvalidAssetIndex)?
            .key;
        let authority_bump = proposal_transaction
            .asset_authority_bumps
            .get(position)
            .ok_or(MultisigError::InvalidAssetIndex)?;

        signer_seeds.push([
            b"authority",
            group_key.as_ref(),
            asset_key.as_ref(),
            authority_bump,
        ]);
    }

    let signer_slices: Vec<&[&[u8]]> = signer_seeds.iter().map(|s| s.as_slice()).collect();

    for serializable in &proposal_transaction.instructions {
        let instruction: Instruction = serializable.into_instruction();
        invoke_signed(&instruction, ctx.remaining_accounts, &signer_slices)?;
    }

    ctx.accounts.proposal.mark_executed()?;

    Ok(())
}

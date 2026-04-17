#![allow(unexpected_cfgs)]

/// A minimal Solana program for integration testing.
///
/// Instruction layout:
///   - The first byte of `instruction_data` encodes the expected number of
///     asset accounts (token_account + mint pairs) passed as remaining accounts.
///   - Pass account pairs as [token_account_0, mint_0, token_account_1, mint_1, …].
///
/// The program logs a summary line and returns success, which lets integration
/// tests verify that the multisig can invoke arbitrary programs with multiple
/// asset authorities via `execute_proposal_transaction`.
#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

solana_program::declare_id!("9uPtVeP3KVq1NqRtjpLLK6CKKjXwxS33k4HXtD5Snwjd");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // First byte = expected number of (token_account, mint) pairs.
    let pair_count = instruction_data
        .first()
        .copied()
        .ok_or(ProgramError::InvalidInstructionData)? as usize;

    let expected_accounts = pair_count * 2;
    if accounts.len() < expected_accounts {
        msg!(
            "multisig_test_helper: expected {} accounts for {} pairs, got {}",
            expected_accounts,
            pair_count,
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let iter = &mut accounts.iter();
    for i in 0..pair_count {
        let token_account = next_account_info(iter)?;
        let mint = next_account_info(iter)?;
        msg!(
            "multisig_test_helper: pair {} token={} mint={}",
            i,
            token_account.key,
            mint.key,
        );
    }

    msg!("multisig_test_helper: processed {} pairs", pair_count);
    Ok(())
}

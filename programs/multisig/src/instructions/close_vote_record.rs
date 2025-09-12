use anchor_lang::{prelude::*, solana_program::system_program};
use crate::state::{
    error::MultisigError,
    group::Group,
    vote::VoteRecord,
};


#[derive(Accounts)]
pub struct CloseNormalVoteRecordInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// CHECK: The proposal account must already be closed
    pub proposal: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"vote_record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref(), 
            &[vote_record.get_asset_index().unwrap()]],
        bump = vote_record.get_account_bump(),
        close = voter
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,
}

/// Close a vote record for a normal proposal, the rent is refunded to the voter
pub fn close_normal_vote_record_handler(
    ctx: Context<CloseNormalVoteRecordInstructionAccounts>,
) -> Result<()> {
    let proposal = &ctx.accounts.proposal;
    let vote_record = &ctx.accounts.vote_record;

    // Proposal account must already be closed
    require!(
        proposal.lamports() == 0 && 
        *proposal.key == system_program::ID && 
        proposal.data_is_empty(),
        MultisigError::ProposalStillActive
    );

    // Check that vote record is tied to this proposal
    require_keys_eq!(
        *vote_record.get_proposal(),
        proposal.key(),
        MultisigError::UnexpectedProposal
    );

    Ok(())
}

#[derive(Accounts)]
pub struct CloseConfigVoteRecordInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// CHECK: The proposal account must already be closed
    pub proposal: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"vote_record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref()],
        bump = vote_record.get_account_bump(),
        close = voter
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,
}

/// Close a vote record for a config proposal, the rent is refunded to the voter
pub fn close_config_vote_record_handler(
    ctx: Context<CloseConfigVoteRecordInstructionAccounts>,
) -> Result<()> {
    let proposal = &ctx.accounts.proposal;
    let vote_record = &ctx.accounts.vote_record;

    // Proposal account must already be closed
    require!(
        proposal.lamports() == 0 && 
        *proposal.key == system_program::ID && 
        proposal.data_is_empty(),
        MultisigError::ProposalStillActive
    );

    // Check that vote record is tied to this proposal
    require_keys_eq!(
        *vote_record.get_proposal(),
        proposal.key(),
        MultisigError::UnexpectedProposal
    );

    Ok(())
}

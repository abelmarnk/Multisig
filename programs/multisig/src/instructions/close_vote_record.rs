use anchor_lang::{prelude::*, solana_program::system_program};
use crate::{NormalProposal, ProposalState, state::{
    error::MultisigError,
    group::Group,
    vote::VoteRecord,
}};


#[derive(Accounts)]
pub struct CloseNormalVoteRecordInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// CHECK: The proposal related to this vote, it might be closed to we don't attempt to load it
    pub proposal: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"vote_record", group.key().as_ref(), proposal.key().as_ref(),
                voter.key().as_ref(), &[vote_record.get_asset_index().unwrap()]],
        bump = vote_record.get_account_bump(),
        close = voter
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,
}

#[inline(always)]// This function is only called once in the handler
fn close_vote_record_checks(
    group:&Account<'_, Group>,
    proposal:&UncheckedAccount<'_> 
)->Result<()>{

    // Check if the proposal is closed
    
    if !(proposal.data_is_empty() && proposal.lamports() == 0 && proposal.owner == &System::id()) {
        let proposal_account: NormalProposal = NormalProposal::try_deserialize(
            &mut &proposal.data.borrow()[..])?;        

        // Ensure the proposal is in a state that allows closing the transaction

        let now = Clock::get()?.unix_timestamp;

        match proposal_account.get_state() {
            ProposalState::Open =>{
                require_gt!(
                    now,
                    proposal_account.get_proposal_deadline_timestamp(),
                    MultisigError::ProposalStillActive
                )
            },
            ProposalState::Passed =>{
                require_gt!(
                    group.get_proposal_index_after_stale(),
                    proposal_account.get_proposal_index(),
                    MultisigError::ProposalStillActive
                );
            },
            ProposalState::Expired | ProposalState::Failed =>{} // Ok
        }
    }

    Ok(())

}

/// Close a vote record for a normal proposal, the rent is refunded to the voter
/// This instruction can only be called by the voter
pub fn close_normal_vote_record_handler(
    ctx: Context<CloseNormalVoteRecordInstructionAccounts>,
) -> Result<()> {
    let group = &ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    close_vote_record_checks(group, proposal)
}

#[derive(Accounts)]
pub struct CloseConfigVoteRecordInstructionAccounts<'info> {
    #[account(
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    /// CHECK: The proposal account may already be closed
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
/// This instruction can only be called by the voter
pub fn close_config_vote_record_handler(
    ctx: Context<CloseConfigVoteRecordInstructionAccounts>,
) -> Result<()> {
    let group = &ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;
    
    close_vote_record_checks(group, proposal)
}

use anchor_lang::prelude::*;

use crate::state::{
    error::MultisigError, group::Group, member::GroupMember, proposal::EmergencyResetProposal,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateEmergencyResetProposalArgs {
    pub proposal_seed: Pubkey,
    pub proposal_deadline_timestamp: i64,
    pub trusted_member_1: Pubkey,
    pub trusted_member_2: Pubkey,
    pub trusted_member_3: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: CreateEmergencyResetProposalArgs)]
pub struct CreateEmergencyResetProposalAccounts<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// Seeds bind membership to group + proposer.
    #[account(
        seeds = [b"member", group.key().as_ref(), proposer.key().as_ref()],
        bump = proposer_group_account.account_bump
    )]
    pub proposer_group_account: Account<'info, GroupMember>,

    #[account(
        init,
        payer = proposer,
        space = 8 + EmergencyResetProposal::INIT_SPACE,
        seeds = [b"emergency-reset", group.key().as_ref(), args.proposal_seed.as_ref()],
        bump,
    )]
    pub proposal: Account<'info, EmergencyResetProposal>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(
    ctx: &Context<CreateEmergencyResetProposalAccounts>,
    args: &CreateEmergencyResetProposalArgs,
) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require!(
        ctx.accounts.proposer_group_account.has_propose(),
        MultisigError::InsufficientPermissions
    );

    require_gt!(
        args.proposal_deadline_timestamp,
        Clock::get()?.unix_timestamp,
        MultisigError::ProposalExpired
    );

    require!(
        args.trusted_member_1 != args.trusted_member_2
            && args.trusted_member_1 != args.trusted_member_3
            && args.trusted_member_2 != args.trusted_member_3,
        MultisigError::TrustedMembersNotUnique
    );

    Ok(())
}

/// Creates an emergency reset proposal.
///
/// Multiple can be open simultaneously via different `proposal_seed` values.
/// Passes iff all members vote For; fails iff all members vote Against.
pub fn create_emergency_reset_proposal_handler(
    ctx: Context<CreateEmergencyResetProposalAccounts>,
    args: CreateEmergencyResetProposalArgs,
) -> Result<()> {
    checks(&ctx, &args)?;

    let group = &mut ctx.accounts.group;
    let proposal = &mut ctx.accounts.proposal;

    proposal.set_inner(EmergencyResetProposal::new(
        ctx.accounts.proposer.key(),
        args.proposal_seed,
        group.key(),
        ctx.bumps.proposal,
        group.get_and_increment_proposal_index()?,
        args.proposal_deadline_timestamp,
        [
            args.trusted_member_1,
            args.trusted_member_2,
            args.trusted_member_3,
        ],
    )?);

    Ok(())
}

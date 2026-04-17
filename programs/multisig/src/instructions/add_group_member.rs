use crate::state::error::*;
use crate::state::{
    group::Group,
    member::GroupMember,
    proposal::{ConfigChange, ConfigProposal, ProposalState},
};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddGroupMemberInstructionArgs {
    pub new_member: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: AddGroupMemberInstructionArgs)]
pub struct AddGroupMemberInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    // Seeds bind proposal to group.
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
        close = proposer,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    /// CHECK: Must match the proposer stored in the proposal; receives closed-account rent.
    #[account(mut)]
    pub proposer: UncheckedAccount<'info>,

    #[account(
        init,
        space = 8 + GroupMember::INIT_SPACE,
        payer = payer,
        seeds = [b"member", group.key().as_ref(), args.new_member.as_ref()],
        bump
    )]
    pub new_group_member: Account<'info, GroupMember>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(ctx: &Context<AddGroupMemberInstructionAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require_keys_eq!(
        ctx.accounts.proposer.key(),
        ctx.accounts.proposal.proposer,
        MultisigError::InvalidProposer
    );

    let now = Clock::get()?.unix_timestamp;

    require_gte!(
        now,
        ctx.accounts.proposal.get_valid_from_timestamp()?,
        MultisigError::ProposalStillTimelocked
    );

    require_gte!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        now,
        MultisigError::ProposalExpired
    );

    require!(
        ctx.accounts.proposal.state == ProposalState::Passed,
        MultisigError::ProposalNotPassed
    );

    require_gte!(
        ctx.accounts.proposal.proposal_index,
        ctx.accounts.group.proposal_index_after_stale,
        MultisigError::ProposalStale
    );

    Ok(())
}

/// Executes a passed AddGroupMember config proposal.
pub fn add_group_member_handler(
    ctx: Context<AddGroupMemberInstructionAccounts>,
    args: AddGroupMemberInstructionArgs,
) -> Result<()> {
    let AddGroupMemberInstructionArgs { new_member } = args;

    checks(&ctx)?;

    let group = &mut ctx.accounts.group;
    let proposal = &ctx.accounts.proposal;

    group.update_stale_proposal_index();

    match &proposal.config_change {
        ConfigChange::AddGroupMember {
            member,
            weight,
            permissions,
        } => {
            require_keys_eq!(*member, new_member, MultisigError::InvalidMember);

            ctx.accounts.new_group_member.set_inner(GroupMember::new(
                new_member,
                group.key(),
                *permissions,
                *weight,
                ctx.bumps.new_group_member,
                group.max_member_weight,
            )?);

            group.increment_member_count()?;
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }

    Ok(())
}

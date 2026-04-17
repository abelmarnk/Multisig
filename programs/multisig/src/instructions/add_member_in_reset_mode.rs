use anchor_lang::prelude::*;

use crate::{
    state::{error::MultisigError, group::Group, member::GroupMember},
    Permissions,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddMemberInResetModeArgs {
    pub new_member: Pubkey,
    pub weight: u32,
    pub permissions: Permissions,
}

#[derive(Accounts)]
#[instruction(args: AddMemberInResetModeArgs)]
pub struct AddMemberInResetModeAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(
        init,
        payer = payer,
        space = 8 + GroupMember::INIT_SPACE,
        seeds = [b"member", group.key().as_ref(), args.new_member.as_ref()],
        bump,
    )]
    pub new_member_account: Account<'info, GroupMember>,

    pub trusted_1: Signer<'info>,
    pub trusted_2: Signer<'info>,
    pub trusted_3: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(
    ctx: &Context<AddMemberInResetModeAccounts>,
    args: &AddMemberInResetModeArgs,
) -> Result<()> {
    require!(ctx.accounts.group.paused, MultisigError::GroupNotPaused);

    require_keys_eq!(
        ctx.accounts.trusted_1.key(),
        ctx.accounts.group.reset_trusted_1,
        MultisigError::InvalidTrustedMember
    );
    require_keys_eq!(
        ctx.accounts.trusted_2.key(),
        ctx.accounts.group.reset_trusted_2,
        MultisigError::InvalidTrustedMember
    );
    require_keys_eq!(
        ctx.accounts.trusted_3.key(),
        ctx.accounts.group.reset_trusted_3,
        MultisigError::InvalidTrustedMember
    );

    require_gt!(args.weight, 0, MultisigError::InvalidMemberWeight);
    require_gte!(
        ctx.accounts.group.max_member_weight,
        args.weight,
        MultisigError::InvalidMemberWeight
    );
    args.permissions.is_valid()?;

    Ok(())
}

/// Adds a group member while paused. All three trusted members must sign.
pub fn add_member_in_reset_mode_handler(
    ctx: Context<AddMemberInResetModeAccounts>,
    args: AddMemberInResetModeArgs,
) -> Result<()> {
    checks(&ctx, &args)?;

    let group = &mut ctx.accounts.group;

    ctx.accounts.new_member_account.set_inner(GroupMember::new(
        args.new_member,
        group.key(),
        args.permissions,
        args.weight,
        ctx.bumps.new_member_account,
        group.max_member_weight,
    )?);

    group.increment_member_count()?;

    Ok(())
}

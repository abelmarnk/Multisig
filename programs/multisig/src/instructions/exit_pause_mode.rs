use anchor_lang::prelude::*;

use crate::{
    state::{error::MultisigError, group::Group},
    utils::FractionalThreshold,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ExitPauseModeArgs {
    pub add_threshold: FractionalThreshold,
    pub not_add_threshold: FractionalThreshold,
    pub remove_threshold: FractionalThreshold,
    pub not_remove_threshold: FractionalThreshold,
    pub change_config_threshold: FractionalThreshold,
    pub not_change_config_threshold: FractionalThreshold,
    pub minimum_member_count: u32,
    pub minimum_vote_count: u32,
    pub max_member_weight: u32,
    pub minimum_timelock: u32,
}

#[derive(Accounts)]
#[instruction(args: ExitPauseModeArgs)]
pub struct ExitPauseModeAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    pub trusted_1: Signer<'info>,
    pub trusted_2: Signer<'info>,
    pub trusted_3: Signer<'info>,
}

#[inline(always)]
fn checks(ctx: &Context<ExitPauseModeAccounts>, args: &ExitPauseModeArgs) -> Result<()> {
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

    let member_count = ctx.accounts.group.member_count;
    require_gt!(member_count, 0, MultisigError::InvalidMemberCount);

    // Same validation as Group::new() against the supplied config values.
    FractionalThreshold::validate_non_overlapping_pair(args.add_threshold, args.not_add_threshold)?;
    FractionalThreshold::validate_non_overlapping_pair(
        args.remove_threshold,
        args.not_remove_threshold,
    )?;
    FractionalThreshold::validate_non_overlapping_pair(
        args.change_config_threshold,
        args.not_change_config_threshold,
    )?;

    require_gt!(
        args.max_member_weight,
        0,
        MultisigError::InvalidMemberWeight
    );
    require_gt!(
        args.minimum_member_count,
        0,
        MultisigError::InvalidMemberCount
    );
    require_gte!(
        member_count,
        args.minimum_member_count,
        MultisigError::InvalidMemberCount
    );
    require_gt!(
        args.minimum_vote_count,
        0,
        MultisigError::InvalidMemberCount
    );
    require_gte!(
        member_count,
        args.minimum_vote_count,
        MultisigError::InvalidMemberCount
    );

    Ok(())
}

#[inline(always)]
fn apply_exit_pause_mode_config(group: &mut Group, args: &ExitPauseModeArgs) {
    group.add_threshold = args.add_threshold;
    group.not_add_threshold = args.not_add_threshold;
    group.remove_threshold = args.remove_threshold;
    group.not_remove_threshold = args.not_remove_threshold;
    group.change_config_threshold = args.change_config_threshold;
    group.not_change_config_threshold = args.not_change_config_threshold;
    group.minimum_member_count = args.minimum_member_count;
    group.minimum_vote_count = args.minimum_vote_count;
    group.max_member_weight = args.max_member_weight;
    group.set_minimum_timelock(args.minimum_timelock);
    group.clear_pause_state();
}

/// Lifts the emergency pause and applies new group configuration values.
///
/// All three trusted members must sign. Validation mirrors group creation.
pub fn exit_pause_mode_handler(
    ctx: Context<ExitPauseModeAccounts>,
    args: ExitPauseModeArgs,
) -> Result<()> {
    checks(&ctx, &args)?;

    apply_exit_pause_mode_config(&mut ctx.accounts.group, &args);

    Ok(())
}

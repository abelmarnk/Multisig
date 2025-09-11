use anchor_lang::prelude::*;

use crate::{
    state::{
        group::Group,
        member::{GroupMember, Permissions},
    },
    utils::fractional_threshold::FractionalThreshold,
};

// Instruction arguments struct for CreateGroupInstructionAccounts
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateGroupInstructionArgs {
    pub group_seed: Pubkey,
    pub add_threshold: FractionalThreshold,
    pub not_add_threshold: FractionalThreshold,
    pub remove_threshold: FractionalThreshold,
    pub not_remove_threshold: FractionalThreshold,
    pub change_config_threshold: FractionalThreshold,
    pub not_change_config_threshold: FractionalThreshold,
    pub minimum_member_count: u32,
    pub minimum_vote_count: u32,
    pub max_member_weight: u32,
    pub member_weights: [u32; 5],
    pub member_permissions: [u8; 5],
    pub default_timelock_offset: u32,
    pub expiry_offset: u32,
}

#[derive(Accounts)]
#[instruction(args: CreateGroupInstructionArgs)]
pub struct CreateGroupInstructionAccounts<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Group::INIT_SPACE,
        seeds = [b"group", args.group_seed.as_ref()],
        bump
    )]
    pub group: Account<'info, Group>,

    // 5 initial members
    pub member_1: SystemAccount<'info>,
    pub member_2: SystemAccount<'info>,
    pub member_3: SystemAccount<'info>,
    pub member_4: SystemAccount<'info>,
    pub member_5: SystemAccount<'info>,

    // Each member has a PDA account storing their membership state
    #[account(
        init,
        payer = payer,
        space = 8 + GroupMember::INIT_SPACE,
        seeds = [b"member", group.key().as_ref(), member_1.key().as_ref()],
        bump
    )]
    pub member_account_1: Account<'info, GroupMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + GroupMember::INIT_SPACE,
        seeds = [b"member", group.key().as_ref(), member_2.key().as_ref()],
        bump
    )]
    pub member_account_2: Account<'info, GroupMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + GroupMember::INIT_SPACE,
        seeds = [b"member", group.key().as_ref(), member_3.key().as_ref()],
        bump
    )]
    pub member_account_3: Account<'info, GroupMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + GroupMember::INIT_SPACE,
        seeds = [b"member", group.key().as_ref(), member_4.key().as_ref()],
        bump
    )]
    pub member_account_4: Account<'info, GroupMember>,

    #[account(
        init,
        payer = payer,
        space = 8 + GroupMember::INIT_SPACE,
        seeds = [b"member", group.key().as_ref(), member_5.key().as_ref()],
        bump
    )]
    pub member_account_5: Account<'info, GroupMember>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_group_handler(
    ctx: Context<CreateGroupInstructionAccounts>,
    args: CreateGroupInstructionArgs,
) -> Result<()> {

    let CreateGroupInstructionArgs {
        group_seed,
        add_threshold,
        not_add_threshold,
        remove_threshold,
        not_remove_threshold,
        change_config_threshold,
        not_change_config_threshold,
        minimum_member_count,
        minimum_vote_count,
        max_member_weight,
        member_weights,
        member_permissions,
        default_timelock_offset,
        expiry_offset,
    } = args;

    let group = &mut ctx.accounts.group;

    // Initialize group
    let new_group = Group::new(
        group_seed,
        add_threshold,
        not_add_threshold,
        remove_threshold,
        not_remove_threshold,
        change_config_threshold,
        not_change_config_threshold,
        minimum_member_count,
        minimum_vote_count,
        max_member_weight,
        5, // initial member count
        default_timelock_offset,
        expiry_offset,
        ctx.bumps.group,
    )?;
    group.set_inner(new_group);


    let member_accounts = [
        &mut ctx.accounts.member_account_1,
        &mut ctx.accounts.member_account_2,
        &mut ctx.accounts.member_account_3,
        &mut ctx.accounts.member_account_4,
        &mut ctx.accounts.member_account_5,
    ];

    let member_account_bumps = [
        ctx.bumps.member_account_1,
        ctx.bumps.member_account_2,
        ctx.bumps.member_account_3,
        ctx.bumps.member_account_4,
        ctx.bumps.member_account_5,
    ];

    let members = [
        &ctx.accounts.member_1,
        &ctx.accounts.member_2,
        &ctx.accounts.member_3,
        &ctx.accounts.member_4,
        &ctx.accounts.member_5,
    ];

    // Initialize all member accounts using iterators
    for ((((account, member), weight), permissions), bump) in member_accounts
        .into_iter()
        .zip(members.into_iter())
        .zip(member_weights.into_iter())
        .zip(member_permissions.into_iter())
        .zip(member_account_bumps.into_iter())
    {
        account.set_inner(GroupMember::new(
            member.key(),
            group.key(),            
            Permissions::new(permissions)?,
            weight,
            bump,
            max_member_weight,
        )?);
    }

    Ok(())
}

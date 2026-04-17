use crate::{state::*, utils::FractionalThreshold};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateConfigProposalInstructionArgs {
    pub proposal_seed: Pubkey,
    pub timelock_offset: u32,
    pub proposal_deadline_timestamp: i64,
    pub config_change: ConfigChange,
}

#[derive(Accounts)]
#[instruction(args: CreateConfigProposalInstructionArgs)]
pub struct CreateConfigProposalInstructionAccounts<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.asset_address.as_ref()],
        bump = asset.account_bump
    )]
    pub asset: Option<Account<'info, Asset>>,

    #[account(
        seeds = [b"member", group.key().as_ref(), proposer.key().as_ref()],
        bump = proposer_group_account.account_bump
    )]
    pub proposer_group_account: Account<'info, GroupMember>,

    #[account(
        init,
        payer = proposer,
        space = 8 + ConfigProposal::INIT_SPACE,
        seeds = [b"proposal", group.key().as_ref(), args.proposal_seed.as_ref()],
        bump,
    )]
    pub proposal: Account<'info, ConfigProposal>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn validate_member_params(
    group: &Account<Group>,
    weight: u32,
    permissions: &Permissions,
) -> Result<()> {
    permissions.is_valid()?;
    require_gt!(weight, 0, MultisigError::InvalidMemberWeight);
    require_gte!(
        group.max_member_weight,
        weight,
        MultisigError::InvalidMemberWeight
    );
    Ok(())
}

#[inline(always)]
fn validate_group_config_change(
    group: &Account<Group>,
    config_change: &ConfigChange,
) -> Result<()> {
    match config_change {
        ConfigChange::AddGroupMember {
            weight,
            permissions,
            ..
        } => validate_member_params(group, *weight, permissions),
        ConfigChange::RemoveGroupMember { .. } => Ok(()),
        ConfigChange::ChangeGroupConfig { config_type } => {
            validate_group_config_type(group, config_type)
        }
        _ => Err(MultisigError::InvalidConfigChange.into()),
    }
}

#[inline(always)]
fn validate_group_config_type(group: &Account<Group>, config_type: &ConfigType) -> Result<()> {
    match config_type {
        ConfigType::AddMember(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(*threshold, group.not_add_threshold)
        }
        ConfigType::NotAddMember(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(group.add_threshold, *threshold)
        }
        ConfigType::RemoveMember(threshold) => FractionalThreshold::validate_non_overlapping_pair(
            *threshold,
            group.not_remove_threshold,
        ),
        ConfigType::NotRemoveMember(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(group.remove_threshold, *threshold)
        }
        ConfigType::ChangeConfig(threshold) => FractionalThreshold::validate_non_overlapping_pair(
            *threshold,
            group.not_change_config_threshold,
        ),
        ConfigType::NotChangeConfig(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(
                group.change_config_threshold,
                *threshold,
            )
        }
        ConfigType::MinimumMemberCount(count) => {
            require_gt!(*count, 0, MultisigError::InvalidMemberCount);
            require_gte!(
                group.member_count,
                *count,
                MultisigError::InvalidMemberCount
            );
            Ok(())
        }
        ConfigType::MinimumVoteCount(count) => {
            require_gt!(*count, 0, MultisigError::InvalidMemberCount);
            require_gte!(
                group.member_count,
                *count,
                MultisigError::InvalidMemberCount
            );
            Ok(())
        }
        ConfigType::MinimumTimelock(_) => Ok(()), // any u32 is valid
        ConfigType::Use(_) | ConfigType::NotUse(_) => {
            Err(MultisigError::UnexpectedConfigChange.into())
        }
    }
}

#[inline(always)]
fn validate_asset_config_type(asset: &Account<Asset>, config_type: &ConfigType) -> Result<()> {
    match config_type {
        ConfigType::AddMember(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(*threshold, asset.not_add_threshold)
        }
        ConfigType::NotAddMember(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(asset.add_threshold, *threshold)
        }
        ConfigType::RemoveMember(threshold) => FractionalThreshold::validate_non_overlapping_pair(
            *threshold,
            asset.not_remove_threshold,
        ),
        ConfigType::NotRemoveMember(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(asset.remove_threshold, *threshold)
        }
        ConfigType::Use(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(*threshold, asset.not_use_threshold)
        }
        ConfigType::NotUse(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(asset.use_threshold, *threshold)
        }
        ConfigType::ChangeConfig(threshold) => FractionalThreshold::validate_non_overlapping_pair(
            *threshold,
            asset.not_change_config_threshold,
        ),
        ConfigType::NotChangeConfig(threshold) => {
            FractionalThreshold::validate_non_overlapping_pair(
                asset.change_config_threshold,
                *threshold,
            )
        }
        ConfigType::MinimumMemberCount(count) => {
            require_gt!(*count, 0, MultisigError::InvalidMemberCount);
            require_gte!(
                asset.member_count,
                *count,
                MultisigError::InvalidMemberCount
            );
            Ok(())
        }
        ConfigType::MinimumVoteCount(count) => {
            require_gt!(*count, 1, MultisigError::InvalidThreshold);
            require_gte!(
                asset.member_count,
                *count,
                MultisigError::InvalidMemberCount
            );
            Ok(())
        }
        ConfigType::MinimumTimelock(_) => Err(MultisigError::UnexpectedConfigChange.into()),
    }
}

#[inline(always)]
fn validate_asset_config_change(
    group: &Account<Group>,
    asset: &Account<Asset>,
    config_change: &ConfigChange,
) -> Result<()> {
    match config_change {
        ConfigChange::AddAssetMember {
            asset_address,
            weight,
            permissions,
            ..
        } => {
            require_keys_eq!(
                *asset_address,
                asset.asset_address,
                MultisigError::InvalidAsset
            );
            validate_member_params(group, *weight, permissions)?;
        }
        ConfigChange::RemoveAssetMember { asset_address, .. } => {
            require_keys_eq!(
                *asset_address,
                asset.asset_address,
                MultisigError::InvalidAsset
            );
        }
        ConfigChange::ChangeAssetConfig { config_type } => {
            validate_asset_config_type(asset, config_type)?;
        }
        _ => return Err(MultisigError::InvalidConfigChange.into()),
    }
    Ok(())
}

#[inline(always)]
fn checks(
    ctx: &Context<CreateConfigProposalInstructionAccounts>,
    args: &CreateConfigProposalInstructionArgs,
) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require_gt!(
        args.proposal_deadline_timestamp,
        Clock::get()?.unix_timestamp,
        MultisigError::ProposalExpired
    );

    require_gte!(
        args.timelock_offset,
        ctx.accounts.group.minimum_timelock,
        MultisigError::TimelockBelowMinimum
    );

    if args.config_change.is_asset_change() {
        let asset = ctx
            .accounts
            .asset
            .as_ref()
            .ok_or(MultisigError::AssetNotProvided)?;

        validate_asset_config_change(&ctx.accounts.group, asset, &args.config_change)?;
    } else {
        validate_group_config_change(&ctx.accounts.group, &args.config_change)?;
    }

    Ok(())
}

/// Creates a config proposal. Requires Propose permission.
pub fn create_config_proposal_handler(
    ctx: Context<CreateConfigProposalInstructionAccounts>,
    args: CreateConfigProposalInstructionArgs,
) -> Result<()> {
    checks(&ctx, &args)?;

    let CreateConfigProposalInstructionArgs {
        proposal_seed,
        timelock_offset,
        config_change,
        proposal_deadline_timestamp,
    } = args;

    let proposer_key = ctx.accounts.proposer.key();
    let group = &mut ctx.accounts.group;
    let proposer_member = &ctx.accounts.proposer_group_account;
    let proposal = &mut ctx.accounts.proposal;

    require!(
        proposer_member.has_propose(),
        MultisigError::InsufficientPermissions
    );

    if config_change.is_group_change() {
        let new_proposal = ConfigProposal::new(
            proposer_key,
            proposal_seed,
            group.key(),
            ctx.bumps.proposal,
            group.get_and_increment_proposal_index()?,
            timelock_offset,
            proposal_deadline_timestamp,
            ProposalTarget::Group,
            config_change,
        )?;

        proposal.set_inner(new_proposal);
        return Ok(());
    } else {
        let asset = ctx
            .accounts
            .asset
            .as_ref()
            .ok_or(MultisigError::AssetNotProvided)?;

        proposal.set_inner(ConfigProposal::new(
            proposer_key,
            proposal_seed,
            group.key(),
            ctx.bumps.proposal,
            group.get_and_increment_proposal_index()?,
            timelock_offset,
            proposal_deadline_timestamp,
            ProposalTarget::Asset(asset.asset_address),
            config_change,
        )?);
    };

    Ok(())
}

use anchor_lang::prelude::*;

use crate::{
    state::{
        asset::Asset,
        error::MultisigError,
        group::Group,
        member::AssetMember,
        proposal::{ConfigProposal, ProposalState, ProposalTarget},
        vote::{VoteChoice, VoteRecord},
    },
    GroupMember,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct VoteOnConfigProposalInstructionArgs {
    pub vote: VoteChoice,
}

#[derive(Accounts)]
#[instruction(args: VoteOnConfigProposalInstructionArgs)]
pub struct VoteOnConfigProposalInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump
    )]
    pub proposal: Account<'info, ConfigProposal>,

    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.asset_address.as_ref()],
        bump = asset.account_bump
    )]
    pub asset: Option<Account<'info, Asset>>,

    #[account(
        mut,
        seeds = [b"member", group.key().as_ref(), voter.key().as_ref()],
        bump = group_member.account_bump
    )]
    pub group_member: Account<'info, GroupMember>,

    #[account(mut)]
    pub asset_member: Option<Account<'info, AssetMember>>,

    #[account(
        init_if_needed,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [b"vote-record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(ctx: &Context<VoteOnConfigProposalInstructionAccounts>) -> Result<()> {
    require!(!ctx.accounts.group.paused, MultisigError::GroupPaused);

    require!(
        ctx.accounts.proposal.state == ProposalState::Open,
        MultisigError::ProposalNotOpen
    );

    require_gte!(
        ctx.accounts.proposal.proposal_index,
        ctx.accounts.group.proposal_index_after_stale,
        MultisigError::ProposalStale
    );

    let now = Clock::get()?.unix_timestamp;

    require_gt!(
        ctx.accounts.proposal.proposal_deadline_timestamp,
        now,
        MultisigError::ProposalExpired
    );

    Ok(())
}

/// Vote on a proposal that changes the configuration of a group or asset if passed.
pub fn vote_on_config_proposal_handler(
    ctx: Context<VoteOnConfigProposalInstructionAccounts>,
    args: VoteOnConfigProposalInstructionArgs,
) -> Result<()> {
    checks(&ctx)?;

    let VoteOnConfigProposalInstructionArgs { vote } = args;

    let proposal = &mut ctx.accounts.proposal;
    let group = &ctx.accounts.group;
    let voter = &ctx.accounts.voter;
    let group_member = &ctx.accounts.group_member;
    let vote_record = &mut ctx.accounts.vote_record;

    // `is_initialized` checks whether `voter != Pubkey::default()`, which is always true once set
    // since the voter is a signer and can never be the default key.
    let is_first_vote = !vote_record.is_initialized();
    if !is_first_vote && vote_record.vote_choice == vote {
        return Ok(());
    }

    if is_first_vote {
        proposal.increment_vote_count()?;
    }

    let target = proposal.target.clone();

    match target {
        ProposalTarget::Group => {
            let weight = group_member.weight.min(group.max_member_weight);
            require_gt!(weight, 0, MultisigError::UnauthorizedVoter);

            if !is_first_vote {
                match vote_record.vote_choice {
                    VoteChoice::For => proposal.sub_weight_for(weight),
                    VoteChoice::Against => proposal.sub_weight_against(weight),
                }
            }

            match vote {
                VoteChoice::For => {
                    proposal.add_weight_for(weight);
                    proposal.check_and_mark_passed(Some(group), None)?;
                }
                VoteChoice::Against => {
                    proposal.add_weight_against(weight);
                    proposal.check_and_mark_failed(Some(group), None)?;
                }
            }
        }
        ProposalTarget::Asset(target_asset) => {
            let asset = ctx
                .accounts
                .asset
                .as_ref()
                .ok_or(MultisigError::AssetNotProvided)?;
            let asset_member = ctx
                .accounts
                .asset_member
                .as_ref()
                .ok_or(MultisigError::AssetMemberNotProvided)?;

            require_keys_eq!(
                target_asset,
                asset.asset_address,
                MultisigError::UnexpectedAsset
            );
            require_keys_eq!(asset_member.user, voter.key(), MultisigError::InvalidMember);
            require_keys_eq!(
                asset_member.group,
                group.key(),
                MultisigError::UnexpectedGroup
            );
            require_keys_eq!(
                asset_member.asset,
                asset.asset_address,
                MultisigError::InvalidAssetMember
            );

            let weight = asset_member.weight.min(group.max_member_weight);
            require_gt!(weight, 0, MultisigError::UnauthorizedVoter);

            if !is_first_vote {
                match vote_record.vote_choice {
                    VoteChoice::For => proposal.sub_weight_for(weight),
                    VoteChoice::Against => proposal.sub_weight_against(weight),
                }
            }

            match vote {
                VoteChoice::For => {
                    proposal.add_weight_for(weight);
                    proposal.check_and_mark_passed(None, Some(asset))?;
                }
                VoteChoice::Against => {
                    proposal.add_weight_against(weight);
                    proposal.check_and_mark_failed(None, Some(asset))?;
                }
            }
        }
    }

    if is_first_vote {
        vote_record.set_inner(VoteRecord::new(
            voter.key(),
            proposal.key(),
            None, // config proposals don't use asset_index
            ctx.bumps.vote_record,
            vote,
        ));
    } else {
        vote_record.vote_choice = vote;
    }

    Ok(())
}

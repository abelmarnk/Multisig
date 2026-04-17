use anchor_lang::prelude::*;

use crate::{
    state::{
        asset::Asset,
        error::MultisigError,
        group::Group,
        member::AssetMember,
        proposal::{NormalProposal, ProposalAssetThresholdState, ProposalState},
        vote::{VoteChoice, VoteRecord},
    },
    GroupMember,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct VoteOnNormalProposalInstructionArgs {
    pub voting_asset_index: u8,
    pub vote: VoteChoice,
}

#[derive(Accounts)]
#[instruction(args: VoteOnNormalProposalInstructionArgs)]
pub struct VoteOnNormalProposalInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.group_seed.as_ref()],
        bump = group.account_bump
    )]
    pub group: Account<'info, Group>,

    /// Seeds checks bind it to the group.
    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.proposal_seed.as_ref()],
        bump = proposal.account_bump,
    )]
    pub proposal: Account<'info, NormalProposal>,

    /// Seeds and ownership checks bind it to the proposal.
    #[account(
        seeds = [b"proposal-transaction", proposal.key().as_ref()],
        bump,
        owner = crate::ID
    )]
    pub proposal_transaction: UncheckedAccount<'info>,

    /// Seeds check binds it to the asset and group.
    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.asset_address.as_ref()],
        bump = asset.account_bump
    )]
    pub asset: Account<'info, Asset>,

    /// Seeds check binds it to the group and voter.
    #[account(
        mut,
        seeds = [b"member", group.key().as_ref(), voter.key().as_ref()],
        bump = group_member.account_bump
    )]
    pub group_member: Account<'info, GroupMember>,

    /// Seeds check binds it to the voter, group and asset.
    #[account(
        mut,
        seeds = [b"asset-member", group.key().as_ref(), asset.asset_address.as_ref(), voter.key().as_ref()],
        bump = asset_member.account_bump
    )]
    pub asset_member: Account<'info, AssetMember>,

    #[account(
        init_if_needed,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [b"vote-record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref(), &[args.voting_asset_index]],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[inline(always)]
fn checks(
    ctx: &Context<VoteOnNormalProposalInstructionAccounts>,
    args: &VoteOnNormalProposalInstructionArgs,
) -> Result<()> {
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

    require_gt!(
        ctx.accounts.proposal.assets.len(),
        usize::from(args.voting_asset_index),
        MultisigError::InvalidAssetIndex
    );

    require_keys_eq!(
        ctx.accounts.proposal.assets[usize::from(args.voting_asset_index)].asset,
        ctx.accounts.asset.asset_address,
        MultisigError::InvalidAsset
    );

    require!(
        ctx.accounts.proposal.assets[usize::from(args.voting_asset_index)].threshold_state
            == ProposalAssetThresholdState::NoThresholdReached,
        MultisigError::StateAlreadyFinalized
    );

    Ok(())
}

/// Vote on a proposal that would execute a transaction and uses assets
/// controlled by the multisig if passed.
/// This instruction can be called by any group member.
pub fn vote_on_normal_proposal_handler(
    ctx: Context<VoteOnNormalProposalInstructionAccounts>,
    args: VoteOnNormalProposalInstructionArgs,
) -> Result<()> {
    checks(&ctx, &args)?;

    let VoteOnNormalProposalInstructionArgs {
        voting_asset_index,
        vote,
    } = args;

    let proposal = &mut ctx.accounts.proposal;
    let asset = &ctx.accounts.asset;
    let voter = &ctx.accounts.voter;
    let asset_member = &ctx.accounts.asset_member;
    let vote_record = &mut ctx.accounts.vote_record;

    let weight = asset_member.weight;
    require_gt!(weight, 0, MultisigError::UnauthorizedVoter);

    let asset_index = usize::from(voting_asset_index);

    // `is_initialized` checks whether `voter != Pubkey::default()`, which is always true once set
    // since the voter is a signer and can never be the default key.
    if !vote_record.is_initialized() {
        // First vote
        proposal
            .get_asset_mut(asset_index)
            .ok_or(MultisigError::InvalidAssetIndex)?
            .increment_vote_count()?;

        match vote {
            VoteChoice::For => {
                proposal
                    .get_asset_mut(asset_index)
                    .ok_or(MultisigError::InvalidAssetIndex)?
                    .add_use_vote_weight(weight);
                proposal.check_and_mark_asset_passed(asset_index, asset)?;
            }
            VoteChoice::Against => {
                proposal
                    .get_asset_mut(asset_index)
                    .ok_or(MultisigError::InvalidAssetIndex)?
                    .add_not_use_vote_weight(weight);
                proposal.check_and_mark_asset_failed(asset_index, asset)?;
            }
        }

        vote_record.set_inner(VoteRecord::new(
            voter.key(),
            proposal.key(),
            Some(voting_asset_index),
            ctx.bumps.vote_record,
            vote,
        ));
    } else {
        // Re-vote

        if vote_record.vote_choice != vote {
            // Undo previous
            match vote_record.vote_choice {
                VoteChoice::For => {
                    proposal
                        .get_asset_mut(asset_index)
                        .ok_or(MultisigError::InvalidAssetIndex)?
                        .sub_use_vote_weight(weight);
                }
                VoteChoice::Against => {
                    proposal
                        .get_asset_mut(asset_index)
                        .ok_or(MultisigError::InvalidAssetIndex)?
                        .sub_not_use_vote_weight(weight);
                }
            }

            // Apply new
            match vote {
                VoteChoice::For => {
                    proposal
                        .get_asset_mut(asset_index)
                        .ok_or(MultisigError::InvalidAssetIndex)?
                        .add_use_vote_weight(weight);
                    proposal.check_and_mark_asset_passed(asset_index, asset)?;
                }
                VoteChoice::Against => {
                    proposal
                        .get_asset_mut(asset_index)
                        .ok_or(MultisigError::InvalidAssetIndex)?
                        .add_not_use_vote_weight(weight);
                    proposal.check_and_mark_asset_failed(asset_index, asset)?;
                }
            }

            vote_record.vote_choice = vote;
        }
    }

    Ok(())
}

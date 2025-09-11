use anchor_lang::prelude::*;

use crate::{
    state::{
        asset::Asset,
        error::MultisigError,
        group::Group,
        member::AssetMember,
        proposal::{ConfigProposal, NormalProposal, ProposalState, ProposalTarget},
        vote::{VoteChoice, VoteRecord},
    },
    GroupMember,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct VoteOnNormalProposalInstructionArgs {
    pub voting_asset_index: u8,
    pub vote: VoteChoice,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct VoteOnConfigProposalInstructionArgs {
    pub vote: VoteChoice,
}

#[derive(Accounts)]
#[instruction(args: VoteOnNormalProposalInstructionArgs)]
pub struct VoteOnNormalProposalInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump(),
    )]
    pub proposal: Account<'info, NormalProposal>,

    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.get_asset_address().as_ref()],
        bump = asset.get_account_bump()
    )]
    pub asset: Account<'info, Asset>,

    #[account(
        mut,
        seeds = [b"asset_member", group.key().as_ref(), asset.get_asset_address().as_ref(), voter.key().as_ref()],
        bump = asset_member.get_account_bump()
    )]
    pub asset_member: Account<'info, AssetMember>,

    #[account(
        init_if_needed,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [b"vote_record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref(), &[args.voting_asset_index]],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn vote_on_normal_proposal_handler(
    ctx: Context<VoteOnNormalProposalInstructionAccounts>,
    args: VoteOnNormalProposalInstructionArgs,
) -> Result<()> {
    // Destructure the arguments
    let VoteOnNormalProposalInstructionArgs {
        voting_asset_index,
        vote,
    } = args;

    let proposal = &mut ctx.accounts.proposal;
    let asset = &ctx.accounts.asset;
    let voter = &ctx.accounts.voter;
    let asset_member = &ctx.accounts.asset_member;
    let vote_record = &mut ctx.accounts.vote_record;

    // Check the proposal is still open
    require!(
        proposal.get_state() == ProposalState::Open,
        MultisigError::ProposalNotOpen
    );

    if Clock::get()?
        .unix_timestamp
        .gt(&proposal.get_expiration_timestamp())
    {
        proposal.set_state(ProposalState::Expired)?;
        return Err(MultisigError::ProposalExpired.into());
    }

    let asset_index = usize::from(voting_asset_index);

    // Validate index
    require!(
        asset_index.lt(&proposal.get_assets().len()),
        MultisigError::InvalidAssetIndex
    );

    // Validate asset and user
    require_keys_eq!(
        *proposal.get_assets()[asset_index].get_asset(),
        *asset.get_asset_address(),
        MultisigError::InvalidAsset
    );

    require_keys_eq!(
        asset_member.get_asset(),
        *asset.get_asset_address(),
        MultisigError::InvalidAssetMember
    );
    require_keys_eq!(
        asset_member.get_user(),
        voter.key(),
        MultisigError::UnauthorizedVoter
    );

    let weight = asset_member.get_weight();

    // First vote or re-vote?
    if vote_record.to_account_info().data_is_empty() {
        // First vote
        match vote {
            VoteChoice::For => {
                proposal
                    .get_asset_mut(asset_index)
                    .unwrap()
                    .add_use_vote_weight(weight);
                proposal.check_and_mark_asset_passed(asset_index, asset)?;
            }
            VoteChoice::Against => {
                proposal
                    .get_asset_mut(asset_index)
                    .unwrap()
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
        require_keys_eq!(
            *vote_record.get_voter(),
            voter.key(),
            MultisigError::UnauthorizedVoter
        );

        if vote_record.get_vote_choice() != vote {
            // Undo previous
            match vote_record.get_vote_choice() {
                VoteChoice::For => {
                    proposal
                        .get_asset_mut(asset_index)
                        .unwrap()
                        .sub_use_vote_weight(weight);
                }
                VoteChoice::Against => {
                    proposal
                        .get_asset_mut(asset_index)
                        .unwrap()
                        .sub_not_use_vote_weight(weight);
                }
            }

            // Apply new
            match vote {
                VoteChoice::For => {
                    proposal
                        .get_asset_mut(asset_index)
                        .unwrap()
                        .add_use_vote_weight(weight);
                    proposal.check_and_mark_asset_passed(asset_index, asset)?;
                }
                VoteChoice::Against => {
                    proposal
                        .get_asset_mut(asset_index)
                        .unwrap()
                        .add_not_use_vote_weight(weight);
                    proposal.check_and_mark_asset_failed(asset_index, asset)?;
                }
            }

            vote_record.set_vote_choice(vote);
        }
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(args: VoteOnConfigProposalInstructionArgs)]
pub struct VoteOnConfigProposalInstructionAccounts<'info> {
    #[account(
        mut,
        seeds = [b"group", group.get_group_seed().as_ref()],
        bump = group.get_account_bump()
    )]
    pub group: Account<'info, Group>,

    #[account(
        mut,
        seeds = [b"proposal", group.key().as_ref(), proposal.get_proposal_seed().as_ref()],
        bump = proposal.get_account_bump()
    )]
    pub proposal: Account<'info, ConfigProposal>,

    #[account(
        mut,
        seeds = [b"asset", group.key().as_ref(), asset.get_asset_address().as_ref()],
        bump = asset.get_account_bump()
    )]
    pub asset: Option<Account<'info, Asset>>,

    #[account(
        mut,
        seeds = [b"member", group.key().as_ref(), voter.key().as_ref()],
        bump = group_member.get_account_bump()
    )]
    pub group_member: Account<'info, GroupMember>,

    #[account(
        mut,
        seeds = [b"asset_member", group.key().as_ref(), 
            asset.as_ref().unwrap().get_asset_address().as_ref(), voter.key().as_ref()],
        bump = asset_member.get_account_bump()
    )]
    pub asset_member: Option<Account<'info, AssetMember>>,

    #[account(
        init_if_needed,
        payer = voter,
        space = 8 + VoteRecord::INIT_SPACE,
        seeds = [b"vote_record", group.key().as_ref(), proposal.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote_record: Account<'info, VoteRecord>,

    #[account(mut)]
    pub voter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn vote_on_config_proposal_handler(
    ctx: Context<VoteOnConfigProposalInstructionAccounts>,
    args: VoteOnConfigProposalInstructionArgs,
) -> Result<()> {
    // Destructure the arguments
    let VoteOnConfigProposalInstructionArgs { vote } = args;

    let proposal = &mut ctx.accounts.proposal;
    let group = &ctx.accounts.group;
    let voter = &ctx.accounts.voter;
    let group_member = &ctx.accounts.group_member;
    let vote_record = &mut ctx.accounts.vote_record;

    // Proposal must be open
    require!(
        proposal.get_state() == ProposalState::Open,
        MultisigError::ProposalNotOpen
    );

    // Expiry check
    if Clock::get()?
        .unix_timestamp
        .gt(&proposal.get_expiration_timestamp())
    {
        proposal.set_state(ProposalState::Expired)?;
        return Err(MultisigError::ProposalExpired.into());
    }

    // Validate voter
    require_keys_eq!(
        *group_member.get_user(),
        voter.key(),
        MultisigError::UnauthorizedVoter
    );
    require_keys_eq!(
        *proposal.get_group(),
        group.key(),
        MultisigError::UnexpectedGroup
    );

    // First vote?
    if vote_record.to_account_info().data_is_empty() {
        match proposal.get_target() {
            ProposalTarget::Group => match vote {
                VoteChoice::For => {
                    proposal.add_weight_for(group_member.get_weight());
                    proposal.check_and_mark_passed(Some(group), None)?;
                }
                VoteChoice::Against => {
                    proposal.add_weight_against(group_member.get_weight());
                    proposal.check_and_mark_failed(Some(group), None)?;
                }
            },
            ProposalTarget::Asset(target_asset) => {
                let asset = ctx
                    .accounts
                    .asset
                    .as_ref()
                    .ok_or(MultisigError::AssetNotProvided)?;
                require_keys_eq!(
                    *target_asset,
                    *asset.get_asset_address(),
                    MultisigError::UnexpectedAsset
                );

                let membership = ctx
                    .accounts
                    .asset_member
                    .as_ref()
                    .ok_or(MultisigError::AssetMemberNotProvided)?;
                require_keys_eq!(
                    membership.get_asset(),
                    *asset.get_asset_address(),
                    MultisigError::InvalidAssetMember
                );
                require_keys_eq!(
                    membership.get_user(),
                    voter.key(),
                    MultisigError::UnauthorizedVoter
                );

                match vote {
                    VoteChoice::For => {
                        proposal.add_weight_for(group_member.get_weight());
                        proposal.check_and_mark_passed(None, Some(asset))?;
                    }
                    VoteChoice::Against => {
                        proposal.add_weight_against(group_member.get_weight());
                        proposal.check_and_mark_failed(None, Some(asset))?;
                    }
                }
            }
        }

        vote_record.set_inner(VoteRecord::new(
            voter.key(),
            proposal.key(),
            None, // config proposals don't use asset_index
            ctx.bumps.vote_record,
            vote,
        ));
    } else {
        // Re-vote
        require_keys_eq!(
            *vote_record.get_voter(),
            voter.key(),
            MultisigError::UnauthorizedVoter
        );

        if vote_record.get_vote_choice() != vote {
            // Undo previous
            match vote_record.get_vote_choice() {
                VoteChoice::For => proposal.sub_weight_for(group_member.get_weight()),
                VoteChoice::Against => proposal.sub_weight_against(group_member.get_weight()),
            }

            // Apply new
            match proposal.get_target() {
                ProposalTarget::Group => match vote {
                    VoteChoice::For => {
                        proposal.add_weight_for(group_member.get_weight());
                        proposal.check_and_mark_passed(Some(group), None)?;
                    }
                    VoteChoice::Against => {
                        proposal.add_weight_against(group_member.get_weight());
                        proposal.check_and_mark_failed(Some(group), None)?;
                    }
                },
                ProposalTarget::Asset(target_asset) => {
                    let asset = ctx
                        .accounts
                        .asset
                        .as_ref()
                        .ok_or(MultisigError::AssetNotProvided)?;
                    require_keys_eq!(
                        *target_asset,
                        *asset.get_asset_address(),
                        MultisigError::UnexpectedAsset
                    );

                    let membership = ctx
                        .accounts
                        .asset_member
                        .as_ref()
                        .ok_or(MultisigError::AssetMemberNotProvided)?;
                    require_keys_eq!(
                        membership.get_asset(),
                        *asset.get_asset_address(),
                        MultisigError::InvalidAssetMember
                    );
                    require_keys_eq!(
                        membership.get_user(),
                        voter.key(),
                        MultisigError::UnauthorizedVoter
                    );

                    match vote {
                        VoteChoice::For => {
                            proposal.add_weight_for(group_member.get_weight());
                            proposal.check_and_mark_passed(None, Some(asset))?;
                        }
                        VoteChoice::Against => {
                            proposal.add_weight_against(group_member.get_weight());
                            proposal.check_and_mark_failed(None, Some(asset))?;
                        }
                    }
                }
            }

            vote_record.set_vote_choice(vote);
        }
    }

    Ok(())
}

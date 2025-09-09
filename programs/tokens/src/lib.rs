use anchor_lang::prelude::*;

pub mod state;
pub use state::*;
pub mod utils;
pub use utils::*;
pub mod instructions;
use instructions::{
    create_group,
    add_member,
    remove_member,
    create_proposal,
    vote_proposal,
    add_asset,
    create_proposal_instruction,
    change_config,
    execute_transaction,
};

declare_id!("HDtNkcgMfN4CARCF4DgFo7BGBqyNjQ6LGNYKwQLkshTR");

#[program]
pub mod group {
    use super::*;

    /// Instruction to add an asset mint
    pub fn add_asset_mint(
        ctx: Context<add_asset::AddAssetMintInstructionAccounts>,
        args: add_asset::AddAssetMintInstructionArgs,
    ) -> Result<()> {
        add_asset::add_asset_mint_handler(ctx, args)
    }

    /// Instruction to add an asset token account
    pub fn add_asset_token(
        ctx: Context<add_asset::AddAssetTokenInstructionAccounts>,
        args: add_asset::AddAssetTokenInstructionArgs,
    ) -> Result<()> {
        add_asset::add_asset_token_handler(ctx, args)
    }

    /// Instruction to add a new group member
    pub fn add_group_member(
        ctx: Context<add_member::AddGroupMemberInstructionAccounts>,
        args: add_member::AddGroupMemberInstructionArgs,
    ) -> Result<()> {
        add_member::add_group_member_handler(ctx, args)
    }

    /// Instruction to add a new asset member
    pub fn add_asset_member(
        ctx: Context<add_member::AddAssetMemberInstructionAccounts>,
        args: add_member::AddAssetMemberInstructionArgs,
    ) -> Result<()> {
        add_member::add_asset_member_handler(ctx, args)
    }

    /// Instruction to change group config
    pub fn change_group_config(
        ctx: Context<change_config::ChangeGroupConfigInstructionAccounts>,
    ) -> Result<()> {
        change_config::change_group_config_handler(ctx)
    }

    /// Instruction to change asset config
    pub fn change_asset_config(
        ctx: Context<change_config::ChangeAssetConfigInstructionAccounts>,
    ) -> Result<()> {
        change_config::change_asset_config_handler(ctx)
    }

    /// Instruction to create a new group
    pub fn create_group(
        ctx: Context<create_group::CreateGroupInstructionAccounts>,
        args: create_group::CreateGroupInstructionArgs,
    ) -> Result<()> {
        create_group::create_group_handler(ctx, args)
    }

    /// Instruction to create a proposal transaction
    pub fn create_proposal_transaction(
        ctx: Context<create_proposal_instruction::CreateProposalTransactionInstructionAccounts>,
        args: create_proposal_instruction::CreateProposalTransactionInstructionArgs,
    ) -> Result<()> {
        create_proposal_instruction::create_proposal_transaction_handler(ctx, args)
    }

    /// Instruction to create a normal proposal
    pub fn create_normal_proposal(
        ctx: Context<create_proposal::CreateNormalProposalInstructionAccounts>,
        args: create_proposal::CreateNormalProposalInstructionArgs,
    ) -> Result<()> {
        create_proposal::create_normal_proposal_handler(ctx, args)
    }

    /// Instruction to create a config proposal
    pub fn create_config_proposal(
        ctx: Context<create_proposal::CreateConfigProposalInstructionAccounts>,
        args: create_proposal::CreateConfigProposalInstructionArgs,
    ) -> Result<()> {
        create_proposal::create_config_proposal_handler(ctx, args)
    }

    /// Instruction to execute a proposal transaction
    pub fn execute_proposal_transaction(
        ctx: Context<execute_transaction::ExecuteProposalTransactionInstructionAccounts>,
    ) -> Result<()> {
        execute_transaction::execute_proposal_transaction_handler(ctx)
    }

    /// Instruction to remove a group member
    pub fn remove_group_member(
        ctx: Context<remove_member::RemoveGroupMemberInstructionAccounts>,
    ) -> Result<()> {
        remove_member::remove_group_member_handler(ctx)
    }

    /// Instruction to remove an asset member
    pub fn remove_asset_member(
        ctx: Context<remove_member::RemoveAssetMemberInstructionAccounts>,
    ) -> Result<()> {
        remove_member::remove_asset_member_handler(ctx)
    }

    /// Instruction to vote on a normal proposal
    pub fn vote_on_normal_proposal(
        ctx: Context<vote_proposal::VoteOnNormalProposalInstructionAccounts>,
        args: vote_proposal::VoteOnNormalProposalInstructionArgs,
    ) -> Result<()> {
        vote_proposal::vote_on_normal_proposal_handler(ctx, args)
    }

    /// Instruction to vote on a config proposal
    pub fn vote_on_config_proposal(
        ctx: Context<vote_proposal::VoteOnConfigProposalInstructionAccounts>,
        args: vote_proposal::VoteOnConfigProposalInstructionArgs,
    ) -> Result<()> {
        vote_proposal::vote_on_config_proposal_handler(ctx, args)
    }
}

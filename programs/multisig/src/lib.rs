use anchor_lang::prelude::*;

pub mod state;
pub use state::*;
pub mod utils;
pub use utils::*;
pub mod instructions;
use instructions::*;

declare_id!("HDtNkcgMfN4CARCF4DgFo7BGBqyNjQ6LGNYKwQLkshTR");

#[program]
pub mod multisig {
    use super::*;

    /// Instruction to add an asset mint
    pub fn add_asset_mint(
        ctx: Context<AddAssetMintInstructionAccounts>,
        args: AddAssetMintInstructionArgs,
    ) -> Result<()> {
        add_asset_mint_handler(ctx, args)
    }

    /// Instruction to add an asset token account
    pub fn add_asset_token(
        ctx: Context<AddAssetTokenInstructionAccounts>,
        args: AddAssetTokenInstructionArgs,
    ) -> Result<()> {
        add_asset_token_handler(ctx, args)
    }

    /// Instruction to add a new group member
    pub fn add_group_member(
        ctx: Context<AddGroupMemberInstructionAccounts>,
        args: AddGroupMemberInstructionArgs,
    ) -> Result<()> {
        add_group_member_handler(ctx, args)
    }

    /// Instruction to add a new asset member
    pub fn add_asset_member(
        ctx: Context<AddAssetMemberInstructionAccounts>,
        args: AddAssetMemberInstructionArgs,
    ) -> Result<()> {
        add_asset_member_handler(ctx, args)
    }

    /// Instruction to change group config
    pub fn change_group_config(ctx: Context<ChangeGroupConfigInstructionAccounts>) -> Result<()> {
        change_group_config_handler(ctx)
    }

    /// Instruction to change asset config
    pub fn change_asset_config(ctx: Context<ChangeAssetConfigInstructionAccounts>) -> Result<()> {
        change_asset_config_handler(ctx)
    }

    /// Instruction to create a new group
    pub fn create_new_group(
        ctx: Context<CreateGroupInstructionAccounts>,
        args: CreateGroupInstructionArgs,
    ) -> Result<()> {
        create_group_handler(ctx, args)
    }

    /// Instruction to create a proposal transaction
    pub fn create_proposal_transaction(
        ctx: Context<CreateProposalTransactionInstructionAccounts>,
        args: CreateProposalTransactionInstructionArgs,
    ) -> Result<()> {
        create_proposal_transaction_handler(ctx, args)
    }

    /// Instruction to create a normal proposal
    pub fn create_normal_proposal(
        ctx: Context<CreateNormalProposalInstructionAccounts>,
        args: CreateNormalProposalInstructionArgs,
    ) -> Result<()> {
        create_normal_proposal_handler(ctx, args)
    }

    /// Instruction to create a config proposal
    pub fn create_config_proposal(
        ctx: Context<CreateConfigProposalInstructionAccounts>,
        args: CreateConfigProposalInstructionArgs,
    ) -> Result<()> {
        create_config_proposal_handler(ctx, args)
    }

    /// Instruction to execute a proposal transaction
    pub fn execute_proposal_transaction(
        ctx: Context<ExecuteProposalTransactionInstructionAccounts>,
    ) -> Result<()> {
        execute_proposal_transaction_handler(ctx)
    }

    /// Instruction to remove a group member
    pub fn remove_group_member(ctx: Context<RemoveGroupMemberInstructionAccounts>) -> Result<()> {
        remove_group_member_handler(ctx)
    }

    /// Instruction to remove an asset member
    pub fn remove_asset_member(ctx: Context<RemoveAssetMemberInstructionAccounts>) -> Result<()> {
        remove_asset_member_handler(ctx)
    }

    /// Instruction to vote on a normal proposal
    pub fn vote_on_normal_proposal(
        ctx: Context<VoteOnNormalProposalInstructionAccounts>,
        args: VoteOnNormalProposalInstructionArgs,
    ) -> Result<()> {
        vote_on_normal_proposal_handler(ctx, args)
    }

    /// Instruction to vote on a config proposal
    pub fn vote_on_config_proposal(
        ctx: Context<VoteOnConfigProposalInstructionAccounts>,
        args: VoteOnConfigProposalInstructionArgs,
    ) -> Result<()> {
        vote_on_config_proposal_handler(ctx, args)
    }
}

#![allow(unexpected_cfgs)]

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

    /// Registers a new token mint that is controlled by the multisig
    pub fn add_asset_mint(
        ctx: Context<AddAssetMintInstructionAccounts>,
        args: AddAssetMintInstructionArgs,
    ) -> Result<()> {
        add_asset_mint_handler(ctx, args)
    }

    /// Registers a new token account that is controlled by the multisig
    pub fn add_asset_token(
        ctx: Context<AddAssetTokenInstructionAccounts>,
        args: AddAssetTokenInstructionArgs,
    ) -> Result<()> {
        add_asset_token_handler(ctx, args)
    }

    /// Adds a group member to a group, storing their key and weight
    ///  and permissions, as well as the group key for indexing.
    pub fn add_group_member(
        ctx: Context<AddGroupMemberInstructionAccounts>,
        args: AddGroupMemberInstructionArgs,
    ) -> Result<()> {
        add_group_member_handler(ctx, args)
    }

    /// Adds a pre-existing group member to govern an existing asset, storing their key and weight
    ///  and permissions, as well as the group key and asset key for indexing.
    pub fn add_asset_member(
        ctx: Context<AddAssetMemberInstructionAccounts>,
        args: AddAssetMemberInstructionArgs,
    ) -> Result<()> {
        add_asset_member_handler(ctx, args)
    }

    /// Updates group-wide configuration (e.g, timelock, thresholds, expiry),
    /// it must be triggered by an approved proposal.
    pub fn change_group_config(ctx: Context<ChangeGroupConfigInstructionAccounts>) -> Result<()> {
        change_group_config_handler(ctx)
    }

    /// Updates asset-wide configuration (e.g, timelock, thresholds, expiry),
    /// it must be triggered by an approved proposal.
    pub fn change_asset_config(ctx: Context<ChangeAssetConfigInstructionAccounts>) -> Result<()> {
        change_asset_config_handler(ctx)
    }

    /// Initializes a new governance group account with its initial configuration, seeds,
    /// and proposal index tracking as well as other state for maintaining the multisig.
    pub fn create_group(
        ctx: Context<CreateGroupInstructionAccounts>,
        args: CreateGroupInstructionArgs,
    ) -> Result<()> {
        create_group_handler(ctx, args)
    }

    /// Create a transaction associated with a particular proposal
    pub fn create_proposal_transaction(
        ctx: Context<CreateProposalTransactionInstructionAccounts>,
        args: CreateProposalTransactionInstructionArgs,
    ) -> Result<()> {
        create_proposal_transaction_handler(ctx, args)
    }

    /// Creates a proposal with a transaction that uses specific assets and requires
    /// meeting a quorom for each individual asset.
    pub fn create_normal_proposal(
        ctx: Context<CreateNormalProposalInstructionAccounts>,
        args: CreateNormalProposalInstructionArgs,
    ) -> Result<()> {
        create_normal_proposal_handler(ctx, args)
    }

    /// Creates a proposal that targets a group or a specific asset and requires
    /// meeting a quorom for that group or asset to change it's config
    pub fn create_config_proposal(
        ctx: Context<CreateConfigProposalInstructionAccounts>,
        args: CreateConfigProposalInstructionArgs,
    ) -> Result<()> {
        create_config_proposal_handler(ctx, args)
    }

    /// Execute a transaction associated with a particular proposal
    pub fn execute_proposal_transaction(
        ctx: Context<ExecuteProposalTransactionInstructionAccounts>,
    ) -> Result<()> {
        execute_proposal_transaction_handler(ctx)
    }

    /// Removes an existing group member once a proposal to remove them has passed,
    /// closes their GroupMember account and sends the rent to the rent_collector.
    pub fn remove_group_member(ctx: Context<RemoveGroupMemberInstructionAccounts>) -> Result<()> {
        remove_group_member_handler(ctx)
    }

    /// Removes an existing asset member once a proposal to remove them has passed,
    /// closes their AssetMember account and sends the rent to the rent_collector.
    /// It is not checked that they have a corresponding group account since one(AssetMember) could
    /// exist without the other(GroupMember).
    pub fn remove_asset_member(ctx: Context<RemoveAssetMemberInstructionAccounts>) -> Result<()> {
        remove_asset_member_handler(ctx)
    }

    /// Vote on a proposal that would execute a transaction and uses assets
    /// controlled by the multisig if passed.
    pub fn vote_on_normal_proposal(
        ctx: Context<VoteOnNormalProposalInstructionAccounts>,
        args: VoteOnNormalProposalInstructionArgs,
    ) -> Result<()> {
        vote_on_normal_proposal_handler(ctx, args)
    }

    /// Vote on a proposal that changes the configuration of a group or asset if passed.
    pub fn vote_on_config_proposal(
        ctx: Context<VoteOnConfigProposalInstructionAccounts>,
        args: VoteOnConfigProposalInstructionArgs,
    ) -> Result<()> {
        vote_on_config_proposal_handler(ctx, args)
    }

    /// Close a proposal transaction that though was finalized after the proposal was passed
    /// and active(no config had changed), execution was delayed till after a config changed
    /// and refund the rent to the proposal
    pub fn close_proposal_transaction_instruction(
        ctx: Context<CloseProposalTransactionInstructionAccounts>,
    ) -> Result<()> {
        close_proposal_transaction_handler(ctx)
    }

    /// Close a config proposal that failed or expired and refund the rent to the proposer
    pub fn close_proposal_instruction(
        ctx: Context<CloseProposalInstructionAccounts>,
    ) -> Result<()> {
        close_proposal_handler(ctx)
    }

    /// Close a normal proposal that failed, expired, or became stale and refund the rent to the proposer
    pub fn close_normal_proposal_instruction(
        ctx: Context<CloseNormalProposalInstructionAccounts>,
    ) -> Result<()> {
        close_normal_proposal_handler(ctx)
    }

    /// Close an asset member account that has had it's group member account removed(by a proposal)
    /// the rent is sent to the rent collector
    pub fn clean_up_asset_member_instruction(
        ctx: Context<CleanUpAssetMemberInstructionAccounts>,
    ) -> Result<()> {
        clean_up_asset_member_handler(ctx)
    }

    /// Close a vote record for a normal proposal, the rent is refunded to the voter
    pub fn close_normal_vote_record_instruction(
        ctx: Context<CloseNormalVoteRecordInstructionAccounts>,
        args: CloseNormalVoteRecordInstructionArgs,
    ) -> Result<()> {
        close_normal_vote_record_handler(ctx, args)
    }

    /// Close a vote record for a config proposal, the rent is refunded to the voter
    pub fn close_config_vote_record_instruction(
        ctx: Context<CloseConfigVoteRecordInstructionAccounts>,
    ) -> Result<()> {
        close_config_vote_record_handler(ctx)
    }

    /// Creates an emergency reset proposal with three designated trusted members.
    /// If all group members vote For it passes; if all vote Against it fails.
    /// No stake checks are bypassed - the proposal simply requires unanimous agreement.
    pub fn create_emergency_reset_proposal(
        ctx: Context<CreateEmergencyResetProposalAccounts>,
        args: CreateEmergencyResetProposalArgs,
    ) -> Result<()> {
        create_emergency_reset_proposal_handler(ctx, args)
    }

    /// Cast or change a vote on an emergency reset proposal.
    pub fn vote_on_emergency_reset_proposal(
        ctx: Context<VoteOnEmergencyResetAccounts>,
        args: VoteOnEmergencyResetArgs,
    ) -> Result<()> {
        vote_on_emergency_reset_handler(ctx, args)
    }

    /// Execute a passed emergency reset proposal, pausing the group and
    /// storing the three trusted member keys for pause-mode governance.
    pub fn execute_emergency_reset(ctx: Context<ExecuteEmergencyResetAccounts>) -> Result<()> {
        execute_emergency_reset_handler(ctx)
    }

    /// Close an emergency reset proposal that has failed or expired, refunding rent.
    pub fn close_emergency_reset_proposal(
        ctx: Context<CloseEmergencyResetProposalAccounts>,
    ) -> Result<()> {
        close_emergency_reset_proposal_handler(ctx)
    }

    /// Close a vote record for an emergency reset proposal, refunding rent to the voter.
    pub fn close_emergency_reset_vote_record(
        ctx: Context<CloseEmergencyResetVoteRecordAccounts>,
    ) -> Result<()> {
        close_emergency_reset_vote_record_handler(ctx)
    }

    /// Add a group member while the group is in emergency pause mode.
    /// Requires all three trusted members to sign.
    pub fn add_member_in_reset_mode(
        ctx: Context<AddMemberInResetModeAccounts>,
        args: AddMemberInResetModeArgs,
    ) -> Result<()> {
        add_member_in_reset_mode_handler(ctx, args)
    }

    /// Remove a group member while the group is in emergency pause mode.
    /// Requires all three trusted members to sign.
    pub fn remove_member_in_reset_mode(
        ctx: Context<RemoveMemberInResetModeAccounts>,
    ) -> Result<()> {
        remove_member_in_reset_mode_handler(ctx)
    }

    /// Lift the emergency pause mode, restoring normal group operation.
    /// Requires all three trusted members to sign and the group to pass validity checks.
    pub fn exit_pause_mode(
        ctx: Context<ExitPauseModeAccounts>,
        args: ExitPauseModeArgs,
    ) -> Result<()> {
        exit_pause_mode_handler(ctx, args)
    }
}

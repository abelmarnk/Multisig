use std::ops::AddAssign;

use crate::{
    state::{error::MultisigError, group::Group, Asset},
    utils::FractionalThreshold,
    Permissions,
};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{prelude::*, solana_program::hash::HASH_BYTES as HASH_BYTES_LENGTH};

#[account]
pub struct NormalProposal {
    pub assets: Vec<ProposalAsset>,
    /// Hashes of each instruction in the proposal transaction, in order.
    pub instruction_hashes: Vec<[u8; HASH_BYTES_LENGTH]>,
    pub propose_timestamp: i64,
    pub proposal_deadline_timestamp: i64,
    pub proposal_passed_timestamp: Option<i64>,
    pub proposal_index: u64,
    pub timelock_offset: u32,
    pub group: Pubkey,
    pub proposer: Pubkey,
    pub proposal_seed: Pubkey,
    pub passed_assets_count: u8,
    pub state: ProposalState,
    pub account_bump: u8,
}

impl NormalProposal {
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn new(
        proposer: Pubkey,
        proposal_seed: Pubkey,
        group: Pubkey,
        assets: Vec<ProposalAsset>,
        account_bump: u8,
        proposal_index: u64,
        proposal_deadline_timestamp: i64,
        instruction_hashes: Vec<[u8; HASH_BYTES_LENGTH]>,
        timelock_offset: u32,
    ) -> Result<Self> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        Ok(Self {
            proposer,
            proposal_seed,
            group,
            assets,
            passed_assets_count: 0,
            propose_timestamp: now,
            timelock_offset,
            proposal_passed_timestamp: None,
            state: ProposalState::Open,
            account_bump,
            proposal_deadline_timestamp,
            proposal_index,
            instruction_hashes,
        })
    }

    #[inline(always)]
    pub fn get_asset_mut(&mut self, index: usize) -> Option<&mut ProposalAsset> {
        self.assets.get_mut(index)
    }

    #[inline(always)]
    pub fn increment_passed_assets_count(&mut self) -> Result<()> {
        self.passed_assets_count = self
            .passed_assets_count
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(())
    }

    #[inline(always)]
    pub fn has_all_assets_passed(&self) -> bool {
        self.passed_assets_count as usize == self.assets.len()
    }

    #[inline(always)]
    pub fn set_proposal_passed_timestamp(&mut self, timestamp: i64) {
        self.proposal_passed_timestamp = Some(timestamp);
    }

    #[inline(always)]
    pub fn get_valid_from_timestamp(&self) -> Result<i64> {
        Ok(self
            .proposal_passed_timestamp
            .ok_or(MultisigError::ProposalNotPassed)?
            .checked_add(i64::from(self.timelock_offset))
            .ok_or(ProgramError::ArithmeticOverflow)?)
    }

    #[inline(always)]
    pub fn set_state(&mut self, new_state: ProposalState) -> Result<()> {
        match self.state {
            ProposalState::Open => {
                self.state = new_state;
                Ok(())
            }
            ProposalState::Passed
            | ProposalState::Failed
            | ProposalState::Expired
            | ProposalState::Executed => Err(error!(MultisigError::InvalidStateTransition)),
        }
    }

    #[inline(always)]
    pub fn mark_executed(&mut self) -> Result<()> {
        match self.state {
            ProposalState::Passed => {
                self.state = ProposalState::Executed;
                Ok(())
            }
            _ => Err(error!(MultisigError::InvalidStateTransition)),
        }
    }

    #[inline(always)]
    pub fn get_size(asset_count: usize, instruction_hash_count: usize) -> usize {
        // assets: Vec<ProposalAsset>
        4 + size_of::<ProposalAsset>() * asset_count
        // instruction_hashes: Vec<[u8; HASH_BYTES_LENGTH]>
        + 4 + HASH_BYTES_LENGTH * instruction_hash_count
        // propose_timestamp: i64
        + size_of::<i64>()
        // proposal_deadline_timestamp: i64
        + size_of::<i64>()
        // proposal_passed_timestamp: Option<i64>
        + size_of::<Option<i64>>()
        // proposal_index: u64
        + size_of::<u64>()
        // timelock_offset: u32
        + size_of::<u32>()
        // group: Pubkey
        + size_of::<Pubkey>()
        // proposer: Pubkey
        + size_of::<Pubkey>()
        // proposal_seed: Pubkey
        + size_of::<Pubkey>()
        // passed_assets_count: u8
        + size_of::<u8>()
        // state: ProposalState
        + size_of::<ProposalState>()
        // account_bump: u8
        + size_of::<u8>()
    }

    /// Check if an asset has enough support to be marked as passed
    /// and pass the proposal if possible
    pub fn check_and_mark_asset_passed(
        &mut self,
        index: usize,
        governed_asset: &Account<Asset>,
    ) -> Result<bool> {
        let asset = &mut self.assets[index];

        // We can only change the state of an asset if it is yet to reach any threshold
        if asset.threshold_state != ProposalAssetThresholdState::NoThresholdReached {
            return Ok(false);
        }

        // Overflow not possible, each weight <= u32::max() and members <= u32::max()
        let total_votes_weight = asset.use_vote_weight + asset.not_use_vote_weight;

        // If the vote count is yet to meet the quorom then the vote cannot pass
        if asset.vote_count.lt(&governed_asset.minimum_vote_count) {
            return Ok(false);
        }

        // Has the passing threshold been reached?
        let passes_threshold = governed_asset
            .use_threshold
            .less_than_or_equal(asset.use_vote_weight, total_votes_weight)?;

        if !passes_threshold {
            return Ok(false);
        }

        // We can use this asset
        asset.set_threshold_state(ProposalAssetThresholdState::UseThresholdReached)?;

        self.increment_passed_assets_count()?;

        // The proposal has been passed since the owners of the asset have voted for use
        if self.has_all_assets_passed() {
            self.state = ProposalState::Passed;
            self.set_proposal_passed_timestamp(Clock::get()?.unix_timestamp);
        }

        // This asset just now passed
        Ok(true)
    }

    /// Check if an asset has enough opposition to be marked as failed
    /// and fail the proposal if possible
    pub fn check_and_mark_asset_failed(
        &mut self,
        index: usize,
        governed_asset: &Account<Asset>,
    ) -> Result<bool> {
        let asset = &mut self.assets[index];

        // We can only change the state of an asset if it is yet to reach any threshold
        if asset.threshold_state != ProposalAssetThresholdState::NoThresholdReached {
            return Ok(false);
        }

        // Overflow not possible: u32::MAX * u32::MAX < u64::MAX
        let total_votes_weight = asset.use_vote_weight + asset.not_use_vote_weight;

        // If the vote count is yet to meet the quorom then the vote cannot pass
        if asset.vote_count.lt(&governed_asset.minimum_vote_count) {
            return Ok(false);
        }

        // Has the failing threshold been reached?
        let fails_threshold = governed_asset
            .not_use_threshold
            .less_than_or_equal(asset.not_use_vote_weight, total_votes_weight)?;

        if !fails_threshold {
            return Ok(false);
        }

        // We cannot use this asset
        asset.set_threshold_state(ProposalAssetThresholdState::NotUseThresholdReached)?;

        // The proposal has failed since the owners of the asset have voted against use
        self.state = ProposalState::Failed;

        // This asset just now failed
        Ok(true)
    }
}

#[account]
#[derive(InitSpace)]
pub struct ConfigProposal {
    pub propose_timestamp: i64,
    pub proposal_deadline_timestamp: i64,
    pub proposal_passed_timestamp: Option<i64>,
    pub proposal_index: u64,
    pub for_weight: u64,
    pub against_weight: u64,

    pub group: Pubkey,
    pub proposer: Pubkey,
    pub proposal_seed: Pubkey,

    pub timelock_offset: u32,
    pub vote_count: u32,

    pub target: ProposalTarget,
    pub config_change: ConfigChange,

    pub state: ProposalState,
    pub account_bump: u8,
}

impl ConfigProposal {
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn new(
        proposer: Pubkey,
        proposal_seed: Pubkey,
        group: Pubkey,
        account_bump: u8,
        proposal_index: u64,
        timelock_offset: u32,
        proposal_deadline_timestamp: i64,
        target: ProposalTarget,
        config_change: ConfigChange,
    ) -> Result<Self> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        Ok(Self {
            proposer,
            proposal_seed,
            group,
            for_weight: 0,
            against_weight: 0,
            vote_count: 0,
            target,
            config_change,
            propose_timestamp: now,
            proposal_deadline_timestamp,
            proposal_passed_timestamp: None,
            timelock_offset,
            state: ProposalState::Open,
            account_bump,
            proposal_index,
        })
    }

    #[inline(always)]
    pub fn set_proposal_passed_timestamp(&mut self, timestamp: i64) {
        self.proposal_passed_timestamp = Some(timestamp);
    }

    #[inline(always)]
    pub fn get_valid_from_timestamp(&self) -> Result<i64> {
        Ok(self
            .proposal_passed_timestamp
            .ok_or(MultisigError::ProposalNotPassed)?
            .checked_add(i64::from(self.timelock_offset))
            .ok_or(ProgramError::ArithmeticOverflow)?)
    }

    #[inline(always)]
    pub fn add_weight_for(&mut self, weight: u32) {
        self.for_weight = self.for_weight.saturating_add(u64::from(weight));
    }

    #[inline(always)]
    pub fn sub_weight_for(&mut self, weight: u32) {
        self.for_weight = self.for_weight.saturating_sub(u64::from(weight));
    }

    #[inline(always)]
    pub fn add_weight_against(&mut self, weight: u32) {
        self.against_weight = self.against_weight.saturating_add(u64::from(weight));
    }

    #[inline(always)]
    pub fn sub_weight_against(&mut self, weight: u32) {
        self.against_weight = self.against_weight.saturating_sub(u64::from(weight));
    }

    #[inline(always)]
    pub fn increment_vote_count(&mut self) -> Result<()> {
        self.vote_count = self
            .vote_count
            .checked_add(1)
            .ok_or(MultisigError::TooManyVotes)?;
        Ok(())
    }

    #[inline(always)]
    pub fn set_state(&mut self, new_state: ProposalState) -> Result<()> {
        match self.state {
            ProposalState::Open => {
                self.state = new_state;
                Ok(())
            }
            ProposalState::Passed
            | ProposalState::Failed
            | ProposalState::Expired
            | ProposalState::Executed => Err(MultisigError::InvalidStateTransition.into()),
        }
    }

    /// Check if a config proposal has enough support to be marked as passed.
    /// Returns true if the proposal was newly passed.
    pub fn check_and_mark_passed(
        &mut self,
        maybe_group: Option<&Account<'_, Group>>,
        maybe_asset: Option<&Account<'_, Asset>>,
    ) -> Result<bool> {
        match &self.target {
            ProposalTarget::Group => {
                let group = maybe_group.ok_or(MultisigError::GroupNotProvided)?;

                // Quorum check
                if self.vote_count.lt(&group.minimum_vote_count) {
                    return Ok(false);
                }

                let total_votes_weight = self.for_weight + self.against_weight;

                let passed_threshold_reached = match &self.config_change {
                    ConfigChange::AddGroupMember { .. } => group
                        .add_threshold
                        .less_than_or_equal(self.for_weight, total_votes_weight)?,
                    ConfigChange::RemoveGroupMember { .. } => group
                        .remove_threshold
                        .less_than_or_equal(self.for_weight, total_votes_weight)?,
                    ConfigChange::ChangeGroupConfig { .. } => group
                        .change_config_threshold
                        .less_than_or_equal(self.for_weight, total_votes_weight)?,
                    _ => return Err(MultisigError::UnexpectedConfigChange.into()),
                };

                if passed_threshold_reached {
                    self.set_state(ProposalState::Passed)?;
                    self.set_proposal_passed_timestamp(Clock::get()?.unix_timestamp);
                }

                Ok(passed_threshold_reached)
            }
            ProposalTarget::Asset(_) => {
                let asset = maybe_asset.ok_or(MultisigError::AssetNotProvided)?;

                // Quorum check
                if self.vote_count.lt(&asset.minimum_vote_count) {
                    return Ok(false);
                }

                let total_votes_weight = self.for_weight + self.against_weight;

                // Check the threshold
                let passed_threshold_reached = match &self.config_change {
                    ConfigChange::AddAssetMember { .. } => asset
                        .add_threshold
                        .less_than_or_equal(self.for_weight, total_votes_weight)?,
                    ConfigChange::RemoveAssetMember { .. } => asset
                        .remove_threshold
                        .less_than_or_equal(self.for_weight, total_votes_weight)?,
                    ConfigChange::ChangeAssetConfig { .. } => asset
                        .change_config_threshold
                        .less_than_or_equal(self.for_weight, total_votes_weight)?,
                    _ => return Err(MultisigError::UnexpectedConfigChange.into()),
                };

                // Set the proposal as passed if the threshold was met
                if passed_threshold_reached {
                    self.set_state(ProposalState::Passed)?;
                    self.set_proposal_passed_timestamp(Clock::get()?.unix_timestamp);
                }

                // Did we just pass?
                Ok(passed_threshold_reached)
            }
        }
    }

    /// Check if a config proposal has enough opposition to be marked as failed.
    /// Returns true if the proposal was newly failed.
    pub fn check_and_mark_failed(
        &mut self,
        maybe_group: Option<&Account<'_, Group>>,
        maybe_asset: Option<&Account<'_, Asset>>,
    ) -> Result<bool> {
        match &self.target {
            ProposalTarget::Group => {
                let group = maybe_group.ok_or(MultisigError::GroupNotProvided)?;

                // Quorum check
                if self.vote_count.lt(&group.minimum_vote_count) {
                    return Ok(false);
                }

                let total_votes_weight = self.for_weight + self.against_weight;

                // Check the threshold
                let failed_threshold_reached = match &self.config_change {
                    ConfigChange::AddGroupMember { .. } => group
                        .not_add_threshold
                        .less_than_or_equal(self.against_weight, total_votes_weight)?,
                    ConfigChange::RemoveGroupMember { .. } => group
                        .not_remove_threshold
                        .less_than_or_equal(self.against_weight, total_votes_weight)?,
                    ConfigChange::ChangeGroupConfig { .. } => group
                        .not_change_config_threshold
                        .less_than_or_equal(self.against_weight, total_votes_weight)?,
                    _ => return Err(MultisigError::UnexpectedConfigChange.into()),
                };

                // Set true if the proposal newly failed
                if failed_threshold_reached {
                    self.set_state(ProposalState::Failed)?;
                }

                // Did we just fail?
                Ok(failed_threshold_reached)
            }
            ProposalTarget::Asset(_) => {
                let asset = maybe_asset.ok_or(MultisigError::AssetNotProvided)?;

                // Quorum check
                if self.vote_count.lt(&asset.minimum_vote_count) {
                    return Ok(false);
                }

                let total_votes_weight = self.for_weight + self.against_weight;

                // Check the threshold
                let failed_threshold_reached = match &self.config_change {
                    ConfigChange::AddAssetMember { .. } => asset
                        .not_add_threshold
                        .less_than_or_equal(self.against_weight, total_votes_weight)?,
                    ConfigChange::RemoveAssetMember { .. } => asset
                        .not_remove_threshold
                        .less_than_or_equal(self.against_weight, total_votes_weight)?,
                    ConfigChange::ChangeAssetConfig { .. } => asset
                        .not_change_config_threshold
                        .less_than_or_equal(self.against_weight, total_votes_weight)?,
                    _ => return Err(MultisigError::UnexpectedConfigChange.into()),
                };

                // Set the proposal as failed if the threshold newly failed
                if failed_threshold_reached {
                    self.set_state(ProposalState::Failed)?;
                }

                // Did we just fail?
                Ok(failed_threshold_reached)
            }
        }
    }
}

/// Stores the different type of changes that could be made to an asset or group
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub enum ConfigChange {
    AddGroupMember {
        member: Pubkey,
        weight: u32,
        permissions: Permissions,
    },
    RemoveGroupMember {
        member: Pubkey,
    },

    AddAssetMember {
        member: Pubkey,
        weight: u32,
        permissions: Permissions,
        asset_address: Pubkey,
    },
    RemoveAssetMember {
        member: Pubkey,
        asset_address: Pubkey,
    },

    ChangeGroupConfig {
        config_type: ConfigType,
    },
    ChangeAssetConfig {
        config_type: ConfigType,
    },
}

impl ConfigChange {
    #[inline]
    pub fn is_asset_change(&self) -> bool {
        matches!(
            self,
            ConfigChange::AddAssetMember { .. }
                | ConfigChange::RemoveAssetMember { .. }
                | ConfigChange::ChangeAssetConfig { .. }
        )
    }

    #[inline(always)]
    pub fn is_group_change(&self) -> bool {
        !self.is_asset_change()
    }
}

/// Stores the state for an existing proposal
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ProposalState {
    Open,
    Passed,
    // Reached the deadline without enough votes
    Failed,
    // Deadline passed before unanimous vote
    Expired,
    Executed,
}

/// Stores whether or not a config proposal is for a group or an asset
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq, Eq)]
pub enum ProposalTarget {
    Group,
    Asset(Pubkey),
}

/// Stores the different type of specifc config changes that could be made to an asset or group
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone)]
pub enum ConfigType {
    AddMember(FractionalThreshold),
    NotAddMember(FractionalThreshold),
    RemoveMember(FractionalThreshold),
    NotRemoveMember(FractionalThreshold),
    Use(FractionalThreshold),
    NotUse(FractionalThreshold),
    MinimumMemberCount(u32),
    MinimumVoteCount(u32),
    ChangeConfig(FractionalThreshold),
    NotChangeConfig(FractionalThreshold),
    MinimumTimelock(u32),
}

/// Locates an asset within the instruction list by specifying which instruction
/// and which account position within that instruction holds the asset key.
#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct AssetIndex {
    pub instruction_index: u8,
    pub account_index: u8,
}

/// Stores the relevant information for a Proposal asset in a transaction
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct ProposalAsset {
    pub use_vote_weight: u64,
    pub not_use_vote_weight: u64,
    pub asset: Pubkey,
    pub vote_count: u32,
    /// Which instruction in the transaction contains the asset key.
    pub instruction_index: u8,
    /// Which account within that instruction holds the asset key.
    pub account_index: u8,
    pub authority_bump: u8,
    pub threshold_state: ProposalAssetThresholdState,
}

impl ProposalAsset {
    #[inline(always)]
    pub fn new(
        instruction_index: u8,
        account_index: u8,
        authority_bump: u8,
        asset: Pubkey,
    ) -> Self {
        Self {
            instruction_index,
            account_index,
            authority_bump,
            asset,
            use_vote_weight: 0,
            not_use_vote_weight: 0,
            vote_count: 0,
            threshold_state: ProposalAssetThresholdState::NoThresholdReached,
        }
    }

    #[inline(always)]
    pub fn increment_vote_count(&mut self) -> Result<()> {
        self.vote_count.add_assign(1);
        Ok(())
    }

    #[inline(always)]
    pub fn decrement_vote_count(&mut self) {
        self.vote_count = self.vote_count.saturating_sub(1);
    }

    #[inline(always)]
    pub fn add_use_vote_weight(&mut self, weight: u32) {
        // Overflow not possible: u32::MAX * u32::MAX < u64::MAX
        self.use_vote_weight.add_assign(u64::from(weight));
    }

    #[inline(always)]
    pub fn sub_use_vote_weight(&mut self, weight: u32) {
        self.use_vote_weight = self.use_vote_weight.saturating_sub(u64::from(weight));
    }

    #[inline(always)]
    pub fn add_not_use_vote_weight(&mut self, weight: u32) {
        // Overflow not possible: u32::MAX * u32::MAX < u64::MAX
        self.not_use_vote_weight.add_assign(u64::from(weight));
    }

    #[inline(always)]
    pub fn sub_not_use_vote_weight(&mut self, weight: u32) {
        self.not_use_vote_weight = self.not_use_vote_weight.saturating_sub(u64::from(weight));
    }

    pub fn set_threshold_state(&mut self, new_state: ProposalAssetThresholdState) -> Result<()> {
        match self.threshold_state {
            ProposalAssetThresholdState::NoThresholdReached => {
                if new_state == ProposalAssetThresholdState::UseThresholdReached
                    || new_state == ProposalAssetThresholdState::NotUseThresholdReached
                {
                    self.threshold_state = new_state;
                    Ok(())
                } else {
                    Err(MultisigError::InvalidStateTransition.into())
                }
            }
            _ => Err(MultisigError::StateAlreadyFinalized.into()),
        }
    }
}

/// Stores a proposal threshold state(e.g whether or not the passing or failing threshold has been met)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, PartialEq, Eq)]
pub enum ProposalAssetThresholdState {
    NoThresholdReached,
    UseThresholdReached,
    NotUseThresholdReached,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct SerailizableAccountMeta {
    pub key: Pubkey,
    pub is_writable: bool,
    pub is_signer: bool,
}

impl SerailizableAccountMeta {
    pub const fn get_size() -> usize {
        32 + // key (Pubkey)
        1 +  // is_writable (bool)
        1 // is_signer (bool)
    }
}

/// Stores an Anchor De/Serializable version of an instruction
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SerializableInstruction {
    pub program_id: Pubkey,
    pub accounts: Vec<SerailizableAccountMeta>,
    pub data: Vec<u8>,
}

impl SerializableInstruction {
    pub fn into_instruction(&self) -> Instruction {
        let metas: Vec<AccountMeta> = self
            .accounts
            .iter()
            .map(|meta| AccountMeta {
                pubkey: meta.key,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect();

        Instruction {
            program_id: self.program_id,
            accounts: metas,
            data: self.data.clone(),
        }
    }

    #[inline(always)]
    pub fn get_size(&self) -> usize {
        32 + // program_id (Pubkey)
        4 + self.accounts.len() * size_of::<SerailizableAccountMeta>() + // accounts (Vec<SerailizableAccountMeta>)
        (4 + self.data.len()) // data (Vec<u8>)
    }
}

// Stores a transaction associated with a particular proposal
#[account]
pub struct ProposalTransaction {
    pub proposal: Pubkey,
    pub group: Pubkey,
    pub proposal_index: u64,
    /// (instruction_index, account_index) pairs - each entry locates the asset key within
    /// the instruction list so the executor can derive the correct authority PDA.
    pub asset_indices: Vec<AssetIndex>,
    pub asset_authority_bumps: Vec<[u8; 1]>,
    /// The ordered list of instructions to execute for this proposal.
    pub instructions: Vec<SerializableInstruction>,
    pub account_bump: u8,
}

impl ProposalTransaction {
    #[inline(always)]
    pub fn new(
        proposal: Pubkey,
        group: Pubkey,
        proposal_index: u64,
        asset_indices: Vec<AssetIndex>,
        asset_authority_bumps: Vec<[u8; 1]>,
        instructions: Vec<SerializableInstruction>,
        account_bump: u8,
    ) -> Self {
        Self {
            proposal,
            group,
            proposal_index,
            asset_indices,
            asset_authority_bumps,
            instructions,
            account_bump,
        }
    }

    /// Calculate the size of ProposalTransaction.
    /// - `asset_len`: number of asset indices (same as number of authority bumps)
    /// - `instructions_total_size`: 4-byte Vec length prefix + sum of each instruction's serialized size
    #[inline(always)]
    pub fn get_size(asset_len: usize, instructions_total_size: usize) -> usize {
        32 + // proposal (Pubkey)
        32 + // group (Pubkey)
        8 +  // proposal_index (u64)
        (4 + asset_len * 2) + // asset_indices (Vec<AssetIndex>) - 2 bytes per entry
        (4 + asset_len) + // asset_authority_bumps (Vec<[u8; 1]>)
        instructions_total_size + // instructions (Vec<SerializableInstruction>)
        1 // account_bump (u8)
    }
}

/// Stale check is deliberately skipped, the proposal survives config changes.
#[account]
#[derive(InitSpace)]
pub struct EmergencyResetProposal {
    pub propose_timestamp: i64,
    pub proposal_deadline_timestamp: i64,
    pub proposal_index: u64,
    pub group: Pubkey,
    pub proposer: Pubkey,
    pub proposal_seed: Pubkey,
    /// The three keys that will govern the group while it is paused.
    pub trusted_members: [Pubkey; 3],
    pub vote_count: u32,
    pub for_count: u32,
    pub against_count: u32,
    pub state: ProposalState,
    pub account_bump: u8,
}

impl EmergencyResetProposal {
    #[inline(always)]
    pub fn new(
        proposer: Pubkey,
        proposal_seed: Pubkey,
        group: Pubkey,
        account_bump: u8,
        proposal_index: u64,
        proposal_deadline_timestamp: i64,
        trusted_members: [Pubkey; 3],
    ) -> Result<Self> {
        let now = Clock::get()?.unix_timestamp;
        Ok(Self {
            group,
            proposer,
            proposal_seed,
            trusted_members,
            propose_timestamp: now,
            proposal_deadline_timestamp,
            proposal_index,
            state: ProposalState::Open,
            vote_count: 0,
            for_count: 0,
            against_count: 0,
            account_bump,
        })
    }

    #[inline(always)]
    pub fn set_state(&mut self, new_state: ProposalState) -> Result<()> {
        match self.state {
            ProposalState::Open => {
                self.state = new_state;
                Ok(())
            }
            _ => Err(MultisigError::InvalidStateTransition.into()),
        }
    }
}

use crate::{
    state::{error::MultisigError, group::Group, Asset},
    utils::FractionalThreshold,
};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{prelude::*, solana_program::hash::HASH_BYTES as HASH_BYTES_LENGTH};

#[account]
pub struct NormalProposal {
    proposer: Pubkey,
    proposal_seed: Pubkey,
    group: Pubkey,
    assets: Vec<ProposalAsset>,
    passed_assets_count: u8,
    propose_timestamp: i64,
    valid_from_timestamp: i64,
    expiration_timestamp: i64,
    state: ProposalState,
    account_bump: u8,
    proposal_index: u64,
    instruction_hash: [u8; HASH_BYTES_LENGTH],
}

impl NormalProposal {
    #[inline(always)]
    pub fn new(
        proposer: Pubkey,
        proposal_seed: Pubkey,
        group: Pubkey,
        assets: Vec<ProposalAsset>,
        account_bump: u8,
        proposal_index: u64,
        instruction_hash: [u8; HASH_BYTES_LENGTH],
        timelock_offset: u32,
        expiry_offset: u32,
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
            valid_from_timestamp: now
                .checked_add(i64::from(timelock_offset))
                .ok_or(ProgramError::ArithmeticOverflow)?,
            expiration_timestamp: now
                .checked_add(i64::from(expiry_offset))
                .ok_or(ProgramError::ArithmeticOverflow)?,
            state: ProposalState::Open,
            account_bump,
            proposal_index,
            instruction_hash,
        })
    }

    #[inline(always)]
    pub fn get_proposer(&self) -> &Pubkey {
        &self.proposer
    }

    #[inline(always)]
    pub fn get_proposal_seed(&self) -> &Pubkey {
        &self.proposal_seed
    }

    #[inline(always)]
    pub fn get_group(&self) -> &Pubkey {
        &self.group
    }

    #[inline(always)]
    pub fn get_assets(&self) -> &Vec<ProposalAsset> {
        &self.assets
    }

    #[inline(always)]
    pub fn get_asset_mut(&mut self, index: usize) -> Option<&mut ProposalAsset> {
        self.assets.get_mut(index)
    }

    #[inline(always)]
    pub fn get_passed_assets_count(&self) -> u8 {
        self.passed_assets_count
    }

    #[inline(always)]
    pub fn increment_passed_assets_count(&mut self) {
        // The maximum no of assets is 10
        self.passed_assets_count = self.passed_assets_count + 1;
    }

    #[inline(always)]
    pub fn has_all_assets_passed(&self) -> bool {
        self.passed_assets_count as usize == self.assets.len()
    }

    #[inline(always)]
    pub fn get_propose_timestamp(&self) -> i64 {
        self.propose_timestamp
    }

    #[inline(always)]
    pub fn get_valid_from_timestamp(&self) -> i64 {
        self.valid_from_timestamp
    }

    #[inline(always)]
    pub fn get_expiration_timestamp(&self) -> i64 {
        self.expiration_timestamp
    }

    #[inline(always)]
    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
    }

    #[inline(always)]
    pub fn get_proposal_index(&self) -> u64 {
        self.proposal_index
    }

    #[inline(always)]
    pub fn get_instruction_hash(&self) -> &[u8; HASH_BYTES_LENGTH] {
        &self.instruction_hash
    }

    #[inline(always)]
    pub fn get_state(&self) -> ProposalState {
        self.state
    }

    #[inline(always)]
    pub fn set_state(&mut self, new_state: ProposalState) -> Result<()> {
        match self.state {
            ProposalState::Open => {
                self.state = new_state;
                Ok(())
            }
            ProposalState::Passed | ProposalState::Failed | ProposalState::Expired => {
                Err(error!(MultisigError::InvalidStateTransition))
            }
        }
    }

    #[inline(always)]
pub fn get_size(asset_count: usize) -> usize {

        // proposer: Pubkey
        size_of::<Pubkey>()
        // proposal_seed: Pubkey
        + size_of::<Pubkey>()
        // group: Pubkey
        + size_of::<Pubkey>()
        // assets: Vec<ProposalAsset> -> 4-byte prefix + elements
        + 4
        + size_of::<ProposalAsset>() * asset_count
        // passed_assets_count: u8
        + size_of::<u8>()
        // propose_timestamp: i64
        + size_of::<i64>()
        // valid_from_timestamp: i64
        + size_of::<i64>()
        // expiration_timestamp: i64
        + size_of::<i64>()
        // state: ProposalState
        + size_of::<ProposalState>()
        // account_bump: u8
        + size_of::<u8>()
        // proposal_index: u64
        + size_of::<u64>()
        // instruction_hash: [u8; HASH_BYTES_LENGTH]
        + HASH_BYTES_LENGTH
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
        if asset.get_threshold_state() != ProposalAssetThresholdState::NoThresholdReached {
            return Ok(false);
        }

        // Overflow not possible, each weight <= u32::max() and members <= u32::max()
        let total_votes_weight = asset.get_use_vote_weight() + asset.get_not_use_vote_weight();

        // If the vote count is yet to meet the quorom then the vote cannot pass
        if asset
            .get_vote_count()
            .lt(&governed_asset.get_minimum_vote_count())
        {
            return Ok(false);
        }

        // Has the passing threshold been reached?
        let passes_threshold = governed_asset
            .get_use_threshold()
            .greater_than_or_equal(asset.get_use_vote_weight(), total_votes_weight)?;

        if !passes_threshold {
            return Ok(false);
        }

        // We can use this asset
        asset.set_threshold_state(ProposalAssetThresholdState::UseThresholdReached)?;

        self.increment_passed_assets_count();

        // The proposal has been passed since the owners of the asset have voted for use
        if self.has_all_assets_passed() {
            self.state = ProposalState::Passed;
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
        if asset.get_threshold_state() != ProposalAssetThresholdState::NoThresholdReached {
            return Ok(false);
        }

        // Overflow not possible, each weight <= u32::max() and members <= u32::max()
        let total_votes_weight = asset.get_use_vote_weight() + asset.get_not_use_vote_weight();

        // If the vote count is yet to meet the quorom then the vote cannot pass
        if asset
            .get_vote_count()
            .lt(&governed_asset.get_minimum_vote_count())
        {
            return Ok(false);
        }

        // Has the failing threshold been reached?
        let fails_threshold = governed_asset
            .get_not_use_threshold()
            .greater_than_or_equal(asset.get_not_use_vote_weight(), total_votes_weight)?;

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
    group: Pubkey,
    proposer: Pubkey,
    proposal_seed: Pubkey,
    target: ProposalTarget,
    propose_timestamp: i64,
    valid_from_timestamp: i64,
    expiration_timestamp: i64,
    proposal_index: u64,
    state: ProposalState,
    vote_count: u32,
    for_weight: u64,
    against_weight: u64,
    config_change: ConfigChange,
    account_bump: u8,
}

impl ConfigProposal {
    #[inline(always)]
    pub fn new(
        proposer: Pubkey,
        proposal_seed: Pubkey,
        group: Pubkey,
        account_bump: u8,
        proposal_index: u64,
        timelock_offset: u32,
        expiry_offset: u32,
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
            valid_from_timestamp: now
                .checked_add(i64::from(timelock_offset))
                .ok_or(ProgramError::ArithmeticOverflow)?,
            expiration_timestamp: now
                .checked_add(i64::from(expiry_offset))
                .ok_or(ProgramError::ArithmeticOverflow)?,
            state: ProposalState::Open,
            account_bump,
            proposal_index,
        })
    }

    #[inline(always)]
    pub fn get_group(&self) -> &Pubkey {
        &self.group
    }

    #[inline(always)]
    pub fn get_proposer(&self) -> &Pubkey {
        &self.proposer
    }

    #[inline(always)]
    pub fn get_proposal_seed(&self) -> &Pubkey {
        &self.proposal_seed
    }

    #[inline(always)]
    pub fn get_target(&self) -> &ProposalTarget {
        &self.target
    }

    #[inline(always)]
    pub fn get_propose_timestamp(&self) -> i64 {
        self.propose_timestamp
    }

    #[inline(always)]
    pub fn get_valid_from_timestamp(&self) -> i64 {
        self.valid_from_timestamp
    }

    #[inline(always)]
    pub fn get_expiration_timestamp(&self) -> i64 {
        self.expiration_timestamp
    }

    #[inline(always)]
    pub fn get_proposal_index(&self) -> u64 {
        self.proposal_index
    }

    #[inline(always)]
    pub fn get_state(&self) -> ProposalState {
        self.state
    }

    #[inline(always)]
    pub fn get_vote_count(&self) -> u32 {
        self.vote_count
    }

    #[inline(always)]
    pub fn get_for_weight(&self) -> u64 {
        self.for_weight
    }

    #[inline(always)]
    pub fn get_against_weight(&self) -> u64 {
        self.against_weight
    }

    #[inline(always)]
    pub fn get_config_change(&self) -> &ConfigChange {
        &self.config_change
    }

    #[inline(always)]
    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
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
    pub fn set_state(&mut self, new_state: ProposalState) -> Result<()> {
        match self.state {
            ProposalState::Open => {
                // Allow only transitions from Open into a finalized state
                self.state = new_state;
                Ok(())
            }
            ProposalState::Passed | ProposalState::Failed | ProposalState::Expired => {
                // Once finalized, state cannot change
                Err(MultisigError::InvalidStateTransition.into())
            }
        }
    }

    /// Check if a config proposal has enough support to be marked as passed.
    /// Returns true if the proposal was newly passed.
    pub fn check_and_mark_passed(
        &mut self,
        maybe_group: Option<&Account<'_, Group>>,
        maybe_asset: Option<&Account<'_, Asset>>,
    ) -> Result<bool> {

        match self.get_target() {
            ProposalTarget::Group => {
                let group = maybe_group.ok_or(MultisigError::GroupNotProvided)?;

                // Quorum check
                if self.get_vote_count().le(&group.get_minimum_vote_count()) {
                    return Ok(false);
                }

                let total_votes_weight = self.get_for_weight() + self.get_against_weight();

                let passed_threshold_reached = match self.get_config_change() {
                    ConfigChange::AddGroupMember { .. } => {
                        group.get_add_threshold().greater_than_or_equal(
                            self.get_for_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::RemoveGroupMember { .. } => {
                        group.get_remove_threshold().greater_than_or_equal(
                            self.get_for_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::ChangeGroupConfig { .. } => {
                        group.get_change_config_threshold().greater_than_or_equal(
                            self.get_for_weight(),
                            total_votes_weight,
                        )?
                    }
                    _ => return Err(MultisigError::UnexpectedConfigChange.into()),
                };

                if passed_threshold_reached {
                    self.set_state(ProposalState::Passed)?;
                }

                Ok(passed_threshold_reached)
            }
            ProposalTarget::Asset(_) => {
                let asset = maybe_asset.ok_or(MultisigError::AssetNotProvided)?;

                // Quorum check
                if self.get_vote_count().le(&asset.get_minimum_vote_count()) {
                    return Ok(false);
                }

                let total_votes_weight = self.get_for_weight() + self.get_against_weight();

                // Check the threshold
                let passed_threshold_reached = match self.get_config_change() {
                    ConfigChange::AddAssetMember { .. } => {
                        asset.get_add_threshold().greater_than_or_equal(
                            self.get_for_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::RemoveAssetMember { .. } => {
                        asset.get_remove_threshold().greater_than_or_equal(
                            self.get_for_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::ChangeAssetConfig { .. } => {
                        asset.get_change_config_threshold().greater_than_or_equal(
                            self.get_for_weight(),
                            total_votes_weight,
                        )?
                    }
                    _ => return Err(MultisigError::UnexpectedConfigChange.into()),
                };

                // Set the proposal as passed if the threshold was met
                if passed_threshold_reached {
                    self.set_state(ProposalState::Passed)?;
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
        if self.get_state() != ProposalState::Open {
            return Ok(false);
        }

        match self.get_target() {
            ProposalTarget::Group => {
                let group = maybe_group.ok_or(MultisigError::GroupNotProvided)?;

                // Quorum check
                if self.get_vote_count().le(&group.get_minimum_vote_count()) {
                    return Ok(false);
                }

                let total_votes_weight = self.get_for_weight() + self.get_against_weight();

                // Check the threshold
                let failed_threshold_reached = match self.get_config_change() {
                    ConfigChange::AddGroupMember { .. } => {
                        group.get_not_add_threshold().greater_than_or_equal(
                            self.get_against_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::RemoveGroupMember { .. } => {
                        group.get_not_remove_threshold().greater_than_or_equal(
                            self.get_against_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::ChangeGroupConfig { .. } => group
                        .get_not_change_config_threshold()
                        .greater_than_or_equal(
                            self.get_against_weight(),
                            total_votes_weight,
                        )?,
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
                if self.get_vote_count().le(&asset.get_minimum_vote_count()) {
                    return Ok(false);
                }

                let total_votes_weight = self.get_for_weight() + self.get_against_weight();

                // Check the threshold
                let failed_threshold_reached = match self.get_config_change() {
                    ConfigChange::AddAssetMember { .. } => {
                        asset.get_not_add_threshold().greater_than_or_equal(
                            self.get_against_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::RemoveAssetMember { .. } => {
                        asset.get_not_remove_threshold().greater_than_or_equal(
                            self.get_against_weight(),
                            total_votes_weight,
                        )?
                    }
                    ConfigChange::ChangeAssetConfig { .. } => asset
                        .get_not_change_config_threshold()
                        .greater_than_or_equal(
                            self.get_against_weight(),
                            total_votes_weight,
                        )?,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub enum ConfigChange {
    AddGroupMember {
        member: Pubkey,
        weight: u32,
        permissions: u8,
    },
    RemoveGroupMember {
        member: Pubkey,
    },

    AddAssetMember {
        member: Pubkey,
        weight: u32,
        permissions: u8,
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
        asset: Pubkey,
        config_type: ConfigType,
    },
}

impl ConfigChange {
    #[inline]
    pub fn is_asset_change(&self) -> bool {
        match self {
            ConfigChange::AddAssetMember { .. } => true,
            ConfigChange::RemoveAssetMember { .. } => true,
            ConfigChange::ChangeAssetConfig { .. } => true,
            _ => false,
        }
    }

    #[inline(always)]
    pub fn is_group_change(&self) -> bool {
        !self.is_asset_change()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ProposalState {
    /// Proposal is active and voting is open
    Open,

    /// Proposal passed successfully
    Passed,

    /// Proposal failed to reach threshold or was rejected
    Failed,

    /// The proposal has run out of time to pass
    Expired,
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, PartialEq, Eq)]
pub enum ProposalTarget {
    Group,
    Asset(Pubkey),
}

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
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct ProposalAsset {
    index: u8,
    authority_bump: u8,
    asset: Pubkey,

    use_vote_weight: u64,     // "For using the asset"
    not_use_vote_weight: u64, // "Against using the asset"

    vote_count: u32,

    // Threshold tracking
    threshold_state: ProposalAssetThresholdState,
}

impl ProposalAsset {
    #[inline(always)]    
    pub fn new(index: u8, authority_bump: u8, asset: Pubkey) -> Self {
        Self {
            index,
            authority_bump,
            asset,
            use_vote_weight: 0,
            not_use_vote_weight: 0,
            vote_count: 0,
            threshold_state: ProposalAssetThresholdState::NoThresholdReached,
        }
    }

    #[inline(always)]    
    pub fn get_index(&self) -> u8 {
        self.index
    }

    #[inline(always)]    
    pub fn get_authority_bump(&self) -> u8 {
        self.authority_bump
    }

    #[inline(always)]    
    pub fn get_asset(&self) -> &Pubkey {
        &self.asset
    }

    #[inline(always)]    
    pub fn get_use_vote_weight(&self) -> u64 {
        self.use_vote_weight
    }

    #[inline(always)]    
    pub fn get_not_use_vote_weight(&self) -> u64 {
        self.not_use_vote_weight
    }

    #[inline(always)]    
    pub fn get_threshold_state(&self) -> ProposalAssetThresholdState {
        self.threshold_state
    }

    #[inline(always)]    
    pub fn increment_vote_count(&mut self) -> Result<()>{
        // No checks here since we check membership before voting and the member
        // count should be sufficiently bounded.
        self.vote_count = self.vote_count + 1;
        Ok(())
    }

    #[inline(always)]    
    pub fn decrement_vote_count(&mut self) {
        self.vote_count = self.vote_count.saturating_sub(1);
    }

    #[inline(always)]    
    pub fn get_vote_count(&self) -> u32 {
        self.vote_count
    }

    #[inline(always)]
    pub fn add_use_vote_weight(&mut self, weight: u32) {
        // No checks here since we check membership before voting and the member
        // count should be sufficiently bounded.
        // Additinally overflow not possible as 
        // u32::max()(max member count) * u32::max()(max weight) < u64::max().
        self.use_vote_weight = self.use_vote_weight + u64::from(weight);
    }

    #[inline(always)]
    pub fn sub_use_vote_weight(&mut self, weight: u32) {
        self.use_vote_weight = self.use_vote_weight.saturating_sub(u64::from(weight));
    }

    #[inline(always)]
    pub fn add_not_use_vote_weight(&mut self, weight: u32) {
        // No checks here since we check membership before voting and the member
        // count should be sufficiently bounded.
        // Additinally overflow not possible as 
        // u32::max()(max member count) * u32::max()(max weight) < u64::max().
        self.not_use_vote_weight = self.not_use_vote_weight + u64::from(weight);
    }

    #[inline(always)]
    pub fn sub_not_use_vote_weight(&mut self, weight: u32) {
        self.not_use_vote_weight = self.not_use_vote_weight.saturating_sub(u64::from(weight));
    }

    pub fn set_threshold_state(&mut self, new_state: ProposalAssetThresholdState) -> Result<()> {
        match self.threshold_state {
            ProposalAssetThresholdState::NoThresholdReached => {
                // Can only move to UseThresholdReached or NotUseThresholdReached
                if new_state == ProposalAssetThresholdState::UseThresholdReached
                    || new_state == ProposalAssetThresholdState::NotUseThresholdReached
                {
                    self.threshold_state = new_state;
                }
                Ok(())
            }
            _ => Err(MultisigError::StateAlreadyFinalized.into()),
        }
    }
}

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
        (4 + self.accounts.len() * SerailizableAccountMeta::get_size()) + // Vec<SerailizableAccountMeta>
        (4 + self.data.len()) // Vec<u8>
    }
}

#[account]
pub struct ProposalTransaction {
    group: Pubkey,
    proposal_index: u64,
    valid_from: i64,
    pub asset_indices: Vec<u8>,
    pub asset_authority_bumps: Vec<[u8; 1]>,
    pub instruction: SerializableInstruction,
    account_bump: u8,
}

impl ProposalTransaction {
    #[inline(always)]    
    pub fn new(
        group: Pubkey,
        proposal_index: u64,
        valid_from: i64,
        asset_indices: Vec<u8>,
        asset_authority_bumps: Vec<[u8; 1]>,
        instruction: SerializableInstruction,
        account_bump: u8,
    ) -> Self {
        Self {
            group,
            proposal_index,
            valid_from,
            asset_indices,
            asset_authority_bumps,
            instruction,
            account_bump,
        }
    }

    #[inline(always)]    
    pub fn get_group(&self) -> Pubkey {
        self.group
    }

    #[inline(always)]    
    pub fn get_proposal_index(&self) -> u64 {
        self.proposal_index
    }

    #[inline(always)]    
    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
    }

    #[inline(always)]
    pub fn get_valid_from(&self) -> i64 {
        self.valid_from
    }

    /// Calculate the size of ProposalTransaction
    /// - `asset_len`: number of asset indices (same as number of authority bumps)
    /// - `instruction_size`: precomputed size of the SerializableInstruction
    #[inline(always)]    
    pub fn get_size(asset_len: usize, instruction_size: usize) -> usize {
        32 + // group (Pubkey)
        8 +  // proposal_index (u64)
        (4 + asset_len * 1) + // asset_indices Vec<u8>
        (4 + asset_len * 1) + // asset_authority_bumps Vec<[u8; 1]>
        4 + instruction_size +    // SerializableInstruction
        1 // account_bump (u8)
    }
}

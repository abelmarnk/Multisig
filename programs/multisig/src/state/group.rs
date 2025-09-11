use crate::{utils::FractionalThreshold, MultisigError};
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Group {
    group_seed: Pubkey,

    /// Threshold to add a member
    add_threshold: FractionalThreshold,
    /// Threshold to stop adding a member
    not_add_threshold: FractionalThreshold,

    /// Threshold to remove a member
    remove_threshold: FractionalThreshold,
    /// Threshold to stop removing a member
    not_remove_threshold: FractionalThreshold,

    /// Threshold to change configuration
    change_config_threshold: FractionalThreshold,
    /// Threshold to stop a configuration change
    not_change_config_threshold: FractionalThreshold,

    /// Minimum number of members required for the group to remain valid
    minimum_member_count: u32,

    minimum_vote_count: u32,

    max_member_weight: u32,

    next_proposal_index: u64,

    proposal_index_after_stale: u64,

    /// Total number of members (tracked by number of NFTs minted)
    member_count: u32,

    // Default timelock
    default_timelock_offset: u32,

    // Time before a proposal times out
    expiry_offset: u32,

    /// PDA bump for the group account itself
    account_bump: u8,
}

impl Group {
    pub fn new(
        group_seed: Pubkey,
        add_threshold: FractionalThreshold,
        mut not_add_threshold: FractionalThreshold,
        remove_threshold: FractionalThreshold,
        mut not_remove_threshold: FractionalThreshold,
        change_config_threshold: FractionalThreshold,
        mut not_change_config_threshold: FractionalThreshold,
        minimum_member_count: u32,
        minimum_vote_count: u32,
        max_member_weight: u32,
        member_count: u32,
        default_timelock_offset: u32,
        expiry_offset: u32,
        account_bump: u8,
    ) -> Result<Self> {
        // Threshold checks
        add_threshold.is_valid()?;
        remove_threshold.is_valid()?;
        change_config_threshold.is_valid()?;
        not_add_threshold.is_valid()?;
        not_remove_threshold.is_valid()?;
        not_change_config_threshold.is_valid()?;

        // Normalize all "not_*" thresholds based on their normal ones
        add_threshold.normalize_other(&mut not_add_threshold)?;
        remove_threshold.normalize_other(&mut not_remove_threshold)?;
        change_config_threshold.normalize_other(&mut not_change_config_threshold)?;

        // Member count checks
        if minimum_vote_count >= member_count {
            return Err(error!(MultisigError::InvalidThreshold));
        }
        if minimum_member_count > member_count {
            return Err(error!(MultisigError::InvalidThreshold));
        }

        // Build group
        let group = Self {
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
            next_proposal_index: 0,
            proposal_index_after_stale: 0,
            member_count,
            default_timelock_offset,
            expiry_offset,
            account_bump
        };

        Ok(group)
    }

        #[inline(always)]
    pub fn get_group_seed(&self) -> &Pubkey {
        &self.group_seed
    }

    #[inline(always)]
    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
    }

    #[inline(always)]
    pub fn get_add_threshold(&self) -> FractionalThreshold {
        self.add_threshold
    }

    pub fn set_add_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.add_threshold = threshold;
        self.add_threshold
            .normalize_other(&mut self.not_add_threshold)?;
        Ok(())
    }

    #[inline(always)]
    pub fn get_not_add_threshold(&self) -> FractionalThreshold {
        self.not_add_threshold
    }

    pub fn set_not_add_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.not_add_threshold = threshold;
        self.add_threshold
            .normalize_other(&mut self.not_add_threshold)?;
        Ok(())
    }

    #[inline(always)]
    pub fn get_remove_threshold(&self) -> FractionalThreshold {
        self.remove_threshold
    }

    pub fn set_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.remove_threshold = threshold;
        self.remove_threshold
            .normalize_other(&mut self.not_remove_threshold)?;
        Ok(())
    }

    #[inline(always)]
    pub fn get_not_remove_threshold(&self) -> FractionalThreshold {
        self.not_remove_threshold
    }

    pub fn set_not_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.not_remove_threshold = threshold;
        self.remove_threshold
            .normalize_other(&mut self.not_remove_threshold)?;
        Ok(())
    }

    #[inline(always)]
    pub fn get_change_config_threshold(&self) -> FractionalThreshold {
        self.change_config_threshold
    }

    pub fn set_change_config_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.change_config_threshold = threshold;
        self.change_config_threshold
            .normalize_other(&mut self.not_change_config_threshold)?;
        Ok(())
    }

    #[inline(always)]
    pub fn get_not_change_config_threshold(&self) -> FractionalThreshold {
        self.not_change_config_threshold
    }

    pub fn set_not_change_config_threshold(
        &mut self,
        threshold: FractionalThreshold,
    ) -> Result<()> {
        threshold.is_valid()?;
        self.not_change_config_threshold = threshold;
        self.change_config_threshold
            .normalize_other(&mut self.not_change_config_threshold)?;
        Ok(())
    }

    #[inline(always)]
    pub fn increment_member_count(&mut self) -> Result<()> {
        self.member_count = self
            .member_count
            .checked_add(1)
            .ok_or(MultisigError::TooManyMembers)?;
        Ok(())
    }

    pub fn decrement_member_count(&mut self) -> Result<()> {
        let new_count = self
            .member_count
            .saturating_sub(1);

        if new_count.le(&self.minimum_vote_count) {
            return Err(MultisigError::InvalidThreshold.into());
        }

        if new_count.lt(&self.minimum_member_count) {
            return Err(MultisigError::InvalidMemberCount.into());
        }

        self.member_count = new_count;

        Ok(())
    }

    #[inline(always)]
    pub fn get_member_count(&self) -> u32 {
        self.member_count
    }

    #[inline(always)]
    pub fn set_minimum_vote_count(&mut self, count: u32) -> Result<()> {
        if count.ge(&self.member_count) {
            return Err(error!(crate::MultisigError::InvalidThreshold));
        }
        self.minimum_vote_count = count;
        Ok(())
    }

    #[inline(always)]
    pub fn get_minimum_vote_count(&self) -> u32 {
        self.minimum_vote_count
    }

    #[inline(always)]
    pub fn set_minimum_member_count(&mut self, count: u32) -> Result<()> {
        if count.lt(&self.member_count) {
            return Err(error!(crate::MultisigError::InvalidMemberCount));
        }
        self.minimum_member_count = count;
        Ok(())
    }

    #[inline(always)]
    pub fn get_minimum_member_count(&self) -> u32 {
        self.minimum_member_count
    }

    #[inline(always)]
    pub fn get_max_member_weight(&self) -> u32 {
        self.max_member_weight
    }

    pub fn get_and_increment_proposal_index(&mut self) -> Result<u64> {
        let current = self.next_proposal_index;
        self.next_proposal_index = self.next_proposal_index.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(current)
    }

    #[inline(always)]
    pub fn get_proposal_index_after_stale(&self) -> u64 {
        self.proposal_index_after_stale
    }

    // This function is called after a config change to invalidate prior proposals
    // and transactions using the previous config
    pub fn set_proposal_index_after_stale(&mut self, proposal_index: u64) {
        self.proposal_index_after_stale = self.next_proposal_index.
            min(self.proposal_index_after_stale.max(proposal_index));
    }

    #[inline(always)]
    pub fn get_timelock_offset(&self) -> u32 {
        self.default_timelock_offset
    }

    #[inline(always)]
    pub fn get_expiry_offset(&self) -> u32 {
        self.expiry_offset
    }

    #[inline(always)]
    pub fn set_timelock_offset(&mut self, offset: u32) {
        self.default_timelock_offset = offset;
    }

    #[inline(always)]
    pub fn set_expiry_offset(&mut self, offset: u32) {
        self.expiry_offset = offset;
    }

}

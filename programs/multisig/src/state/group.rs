use crate::{MultisigError, utils::FractionalThreshold};
use anchor_lang::prelude::*;

/// Stores information required to govern a group
#[account]
#[derive(InitSpace)]
pub struct Group {
    group_seed: Pubkey,

    rent_collector:Pubkey,

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

    /// Total number of members
    member_count: u32,

    /// PDA bump for the group account itself
    account_bump: u8,
}

impl Group {
    pub fn new(
        group_seed: Pubkey,
        rent_collector: Pubkey,
        add_threshold: FractionalThreshold,
        not_add_threshold: FractionalThreshold,
        remove_threshold: FractionalThreshold,
        not_remove_threshold: FractionalThreshold,
        change_config_threshold: FractionalThreshold,
        not_change_config_threshold: FractionalThreshold,
        minimum_member_count: u32,
        minimum_vote_count: u32,
        max_member_weight: u32,
        member_count: u32,
        account_bump: u8,
    ) -> Result<Self> {
        // Threshold checks
        add_threshold.is_valid()?;
        remove_threshold.is_valid()?;
        change_config_threshold.is_valid()?;
        not_add_threshold.is_valid()?;
        not_remove_threshold.is_valid()?;
        not_change_config_threshold.is_valid()?;

        require_gt!(member_count, 0, MultisigError::InvalidMemberCount);
        require_gt!(member_count, minimum_vote_count, MultisigError::InvalidMemberCount);
        require_gte!(member_count, minimum_member_count, MultisigError::InvalidMemberCount);

        let group = Self {
            group_seed,
            rent_collector,
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
            account_bump
        };

        Ok(group)
    }

    #[inline(always)]
    pub fn get_group_seed(&self) -> &Pubkey {
        &self.group_seed
    }

    #[inline(always)]
    pub fn get_rent_collector(&self) -> &Pubkey{
        &self.rent_collector
    }

    #[inline(always)]
    pub fn set_rent_collector(&mut self, rent_collector: Pubkey){
        self.rent_collector = rent_collector;
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
        Ok(())
    }

    #[inline(always)]
    pub fn get_not_add_threshold(&self) -> FractionalThreshold {
        self.not_add_threshold
    }

    pub fn set_not_add_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.not_add_threshold = threshold;
        Ok(())
    }

    #[inline(always)]
    pub fn get_remove_threshold(&self) -> FractionalThreshold {
        self.remove_threshold
    }

    pub fn set_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.remove_threshold = threshold;
        Ok(())
    }

    #[inline(always)]
    pub fn get_not_remove_threshold(&self) -> FractionalThreshold {
        self.not_remove_threshold
    }

    pub fn set_not_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.not_remove_threshold = threshold;
        Ok(())
    }

    #[inline(always)]
    pub fn get_change_config_threshold(&self) -> FractionalThreshold {
        self.change_config_threshold
    }

    pub fn set_change_config_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.change_config_threshold = threshold;
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

        if new_count.le(&self.minimum_vote_count) || new_count.lt(&self.minimum_member_count){
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
            return Err(crate::MultisigError::InvalidMemberCount.into());
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
        if count.gt(&self.member_count) {
            return Err(crate::MultisigError::InvalidMemberCount.into());
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

    /// This function is called after a config change to invalidate prior proposals
    /// and transactions using the previous config
    #[inline(always)]
    pub fn update_stale_proposal_index(&mut self) {
        self.proposal_index_after_stale = self.next_proposal_index;
    }


}

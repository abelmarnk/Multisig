use crate::{utils::FractionalThreshold, MultisigError};
use anchor_lang::prelude::*;

/// Stores information required to govern a group
#[account]
#[derive(InitSpace)]
pub struct Group {
    pub next_proposal_index: u64,
    pub proposal_index_after_stale: u64,

    pub add_threshold: FractionalThreshold,
    pub not_add_threshold: FractionalThreshold,
    pub remove_threshold: FractionalThreshold,
    pub not_remove_threshold: FractionalThreshold,
    pub change_config_threshold: FractionalThreshold,
    pub not_change_config_threshold: FractionalThreshold,

    pub group_seed: Pubkey,
    pub rent_collector: Pubkey,

    /// Keys authorised to operate during pause mode; cleared on exit.
    pub reset_trusted_1: Pubkey,
    pub reset_trusted_2: Pubkey,
    pub reset_trusted_3: Pubkey,

    pub minimum_member_count: u32,
    pub minimum_vote_count: u32,
    pub max_member_weight: u32,
    pub member_count: u32,
    pub minimum_timelock: u32,

    pub paused: bool,
    pub account_bump: u8,
}

impl Group {
    #[inline(always)]
    fn validate_minimum_vote_count(member_count: u32, count: u32) -> Result<()> {
        require_gt!(count, 0, MultisigError::InvalidMemberCount);
        require_gte!(member_count, count, MultisigError::InvalidMemberCount);
        Ok(())
    }

    #[inline(always)]
    fn validate_minimum_member_count(member_count: u32, count: u32) -> Result<()> {
        require_gt!(count, 0, MultisigError::InvalidMemberCount);
        require_gte!(member_count, count, MultisigError::InvalidMemberCount);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
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
        minimum_timelock: u32,
        member_count: u32,
        account_bump: u8,
    ) -> Result<Self> {
        // Threshold checks
        FractionalThreshold::validate_non_overlapping_pair(add_threshold, not_add_threshold)?;
        FractionalThreshold::validate_non_overlapping_pair(remove_threshold, not_remove_threshold)?;
        FractionalThreshold::validate_non_overlapping_pair(
            change_config_threshold,
            not_change_config_threshold,
        )?;

        require_gt!(member_count, 0, MultisigError::InvalidMemberCount);
        require_gt!(max_member_weight, 0, MultisigError::InvalidMemberWeight);
        Self::validate_minimum_vote_count(member_count, minimum_vote_count)?;
        Self::validate_minimum_member_count(member_count, minimum_member_count)?;

        let group = Self {
            next_proposal_index: 0,
            proposal_index_after_stale: 0,
            add_threshold,
            not_add_threshold,
            remove_threshold,
            not_remove_threshold,
            change_config_threshold,
            not_change_config_threshold,
            minimum_member_count,
            minimum_vote_count,
            max_member_weight,
            member_count,
            minimum_timelock,
            group_seed,
            rent_collector,
            reset_trusted_1: Pubkey::default(),
            reset_trusted_2: Pubkey::default(),
            reset_trusted_3: Pubkey::default(),
            paused: false,
            account_bump,
        };

        Ok(group)
    }

    pub fn set_add_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(threshold, self.not_add_threshold)?;
        self.add_threshold = threshold;
        Ok(())
    }

    pub fn set_not_add_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(self.add_threshold, threshold)?;
        self.not_add_threshold = threshold;
        Ok(())
    }

    pub fn set_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(threshold, self.not_remove_threshold)?;
        self.remove_threshold = threshold;
        Ok(())
    }

    pub fn set_not_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(self.remove_threshold, threshold)?;
        self.not_remove_threshold = threshold;
        Ok(())
    }

    pub fn set_change_config_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(
            threshold,
            self.not_change_config_threshold,
        )?;
        self.change_config_threshold = threshold;
        Ok(())
    }

    pub fn set_not_change_config_threshold(
        &mut self,
        threshold: FractionalThreshold,
    ) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(
            self.change_config_threshold,
            threshold,
        )?;
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
        let new_count = self.member_count.saturating_sub(1);

        if new_count.lt(&self.minimum_vote_count) || new_count.lt(&self.minimum_member_count) {
            return Err(MultisigError::InvalidMemberCount.into());
        }

        self.member_count = new_count;

        Ok(())
    }

    #[inline(always)]
    pub fn set_minimum_vote_count(&mut self, count: u32) -> Result<()> {
        Self::validate_minimum_vote_count(self.member_count, count)?;
        self.minimum_vote_count = count;
        Ok(())
    }

    #[inline(always)]
    pub fn set_minimum_member_count(&mut self, count: u32) -> Result<()> {
        Self::validate_minimum_member_count(self.member_count, count)?;
        self.minimum_member_count = count;
        Ok(())
    }

    #[inline(always)]
    pub fn set_minimum_timelock(&mut self, timelock: u32) {
        self.minimum_timelock = timelock;
    }

    /// Decrement member count without enforcing minimum thresholds.
    #[inline(always)]
    pub fn force_decrement_member_count(&mut self) {
        self.member_count = self.member_count.saturating_sub(1);
    }

    pub fn get_and_increment_proposal_index(&mut self) -> Result<u64> {
        let current = self.next_proposal_index;
        self.next_proposal_index = self
            .next_proposal_index
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(current)
    }

    /// Marks all proposals with index < next_proposal_index as stale.
    #[inline(always)]
    pub fn update_stale_proposal_index(&mut self) {
        self.proposal_index_after_stale = self.next_proposal_index;
    }

    /// Pause the group and record the three trusted keys.
    #[inline(always)]
    pub fn pause_group(&mut self, trusted_members: [Pubkey; 3]) {
        self.reset_trusted_1 = trusted_members[0];
        self.reset_trusted_2 = trusted_members[1];
        self.reset_trusted_3 = trusted_members[2];
        self.paused = true;
    }

    /// Clear pause state and wipe trusted keys.
    #[inline(always)]
    pub fn clear_pause_state(&mut self) {
        self.paused = false;
        self.reset_trusted_1 = Pubkey::default();
        self.reset_trusted_2 = Pubkey::default();
        self.reset_trusted_3 = Pubkey::default();
    }
}

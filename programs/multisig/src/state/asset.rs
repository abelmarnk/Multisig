use anchor_lang::prelude::*;

use crate::{utils::FractionalThreshold, MultisigError};

/// Stores information required to govern an asset
#[account]
#[derive(InitSpace)]
pub struct Asset {
    pub asset_address: Pubkey,

    pub use_threshold: FractionalThreshold,
    pub not_use_threshold: FractionalThreshold,

    pub add_threshold: FractionalThreshold,
    pub not_add_threshold: FractionalThreshold,
    pub remove_threshold: FractionalThreshold,
    pub not_remove_threshold: FractionalThreshold,
    pub change_config_threshold: FractionalThreshold,
    pub not_change_config_threshold: FractionalThreshold,

    pub member_count: u32,

    /// Constraints
    pub minimum_member_count: u32,
    pub minimum_vote_count: u32,

    /// PDA bumps
    pub account_bump: u8,
    pub authority_bump: u8,
}

impl Asset {
    #[inline(always)]
    fn validate_minimum_vote_count(member_count: u32, count: u32) -> Result<()> {
        require_gt!(count, 1, MultisigError::InvalidThreshold);
        require_gte!(member_count, count, MultisigError::InvalidMemberCount);
        Ok(())
    }

    #[inline(always)]
    fn validate_minimum_member_count(member_count: u32, count: u32) -> Result<()> {
        require_gt!(count, 0, MultisigError::InvalidMemberCount);
        require_gte!(member_count, count, MultisigError::InvalidMemberCount);
        Ok(())
    }

    /// Create a new Asset with validation
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn new(
        asset_address: Pubkey,
        use_threshold: FractionalThreshold,
        not_use_threshold: FractionalThreshold,
        add_threshold: FractionalThreshold,
        not_add_threshold: FractionalThreshold,
        remove_threshold: FractionalThreshold,
        not_remove_threshold: FractionalThreshold,
        change_config_threshold: FractionalThreshold,
        not_change_config_threshold: FractionalThreshold,
        minimum_member_count: u32,
        minimum_vote_count: u32,
        initial_member_count: u32,
        account_bump: u8,
        authority_bump: u8,
    ) -> Result<Self> {
        Self::validate_minimum_vote_count(initial_member_count, minimum_vote_count)?;
        Self::validate_minimum_member_count(initial_member_count, minimum_member_count)?;

        FractionalThreshold::validate_non_overlapping_pair(use_threshold, not_use_threshold)?;
        FractionalThreshold::validate_non_overlapping_pair(add_threshold, not_add_threshold)?;
        FractionalThreshold::validate_non_overlapping_pair(remove_threshold, not_remove_threshold)?;
        FractionalThreshold::validate_non_overlapping_pair(
            change_config_threshold,
            not_change_config_threshold,
        )?;

        Ok(Self {
            asset_address,
            use_threshold,
            not_use_threshold,
            add_threshold,
            not_add_threshold,
            remove_threshold,
            not_remove_threshold,
            change_config_threshold,
            not_change_config_threshold,
            member_count: initial_member_count,
            minimum_member_count,
            minimum_vote_count,
            account_bump,
            authority_bump,
        })
    }

    pub fn set_use_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(threshold, self.not_use_threshold)?;
        self.use_threshold = threshold;
        Ok(())
    }

    pub fn set_not_use_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        FractionalThreshold::validate_non_overlapping_pair(self.use_threshold, threshold)?;
        self.not_use_threshold = threshold;
        Ok(())
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

    pub fn increment_member_count(&mut self) -> Result<()> {
        self.member_count = self
            .member_count
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
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
}

use anchor_lang::prelude::*;

use crate::{utils::FractionalThreshold, MultisigError};

#[account]
#[derive(InitSpace)]
pub struct Asset {
    asset_address: Pubkey,

    use_threshold: FractionalThreshold,
    not_use_threshold: FractionalThreshold,

    add_threshold: FractionalThreshold,
    not_add_threshold: FractionalThreshold,
    remove_threshold: FractionalThreshold,
    not_remove_threshold: FractionalThreshold,
    change_config_threshold: FractionalThreshold,
    not_change_config_threshold: FractionalThreshold,

    member_count: u32,

    // Constraints
    minimum_member_count: u32,
    minimum_vote_count: u32,

    // PDA bumps
    account_bump: u8,
    authority_bump: u8,
}

impl Asset {
    /// Create a new Asset with validation
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
        account_bump: u8,
        authority_bump: u8,
    ) -> Result<Self> {
        require_gt!(minimum_vote_count, 0, MultisigError::InvalidThreshold);
        require_gt!(minimum_member_count, 0, MultisigError::InvalidMemberCount);

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
            member_count: 0,
            minimum_member_count,
            minimum_vote_count,
            account_bump,
            authority_bump,
        })
    }

    #[inline(always)]
    pub fn get_asset_address(&self) -> &Pubkey {
        &self.asset_address
    }

    #[inline(always)]
    pub fn get_use_threshold(&self) -> FractionalThreshold {
        self.use_threshold
    }

    #[inline(always)]
    pub fn get_not_use_threshold(&self) -> FractionalThreshold {
        self.not_use_threshold
    }

    #[inline(always)]
    pub fn get_add_threshold(&self) -> FractionalThreshold {
        self.add_threshold
    }

    #[inline(always)]
    pub fn get_not_add_threshold(&self) -> FractionalThreshold {
        self.not_add_threshold
    }

    #[inline(always)]
    pub fn get_remove_threshold(&self) -> FractionalThreshold {
        self.remove_threshold
    }

    #[inline(always)]
    pub fn get_not_remove_threshold(&self) -> FractionalThreshold {
        self.not_remove_threshold
    }

    #[inline(always)]
    pub fn get_change_config_threshold(&self) -> FractionalThreshold {
        self.change_config_threshold
    }

    #[inline(always)]
    pub fn get_not_change_config_threshold(&self) -> FractionalThreshold {
        self.not_change_config_threshold
    }

    pub fn set_use_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.use_threshold = threshold;
        self.use_threshold
            .normalize_other(&mut self.not_use_threshold)?;
        Ok(())
    }

    pub fn set_not_use_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.not_use_threshold = threshold;
        self.use_threshold
            .normalize_other(&mut self.not_use_threshold)?;
        Ok(())
    }

    pub fn set_add_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.add_threshold = threshold;
        self.add_threshold
            .normalize_other(&mut self.not_add_threshold)?;
        Ok(())
    }

    pub fn set_not_add_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.not_add_threshold = threshold;
        self.add_threshold
            .normalize_other(&mut self.not_add_threshold)?;
        Ok(())
    }

    pub fn set_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.remove_threshold = threshold;
        self.remove_threshold
            .normalize_other(&mut self.not_remove_threshold)?;
        Ok(())
    }

    pub fn set_not_remove_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.not_remove_threshold = threshold;
        self.remove_threshold
            .normalize_other(&mut self.not_remove_threshold)?;
        Ok(())
    }

    pub fn set_change_config_threshold(&mut self, threshold: FractionalThreshold) -> Result<()> {
        threshold.is_valid()?;
        self.change_config_threshold = threshold;
        self.change_config_threshold
            .normalize_other(&mut self.not_change_config_threshold)?;
        Ok(())
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
    pub fn get_minimum_member_count(&self) -> u32 { 
        self.minimum_member_count 
    }

    #[inline(always)]
    pub fn get_minimum_vote_count(&self) -> u32 { 
        self.minimum_vote_count 
    }

    #[inline(always)]
    pub fn get_account_bump(&self) -> u8 { 
        self.account_bump 
    }

    #[inline(always)]
    pub fn get_authority_bump(&self) -> u8 { 
        self.authority_bump 
    }

    pub fn increment_member_count(&mut self) -> Result<()> {
        self.member_count = self
            .member_count
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        Ok(())
    }

    #[inline(always)]
    pub fn decrement_member_count(&mut self) {
        self.member_count = self.member_count.saturating_sub(1);
    }

    #[inline(always)]
    pub fn get_member_count(&self) -> u32 { 
        self.member_count 
    }

    #[inline(always)]
    pub fn set_minimum_vote_count(&mut self, count: u32) -> Result<()> {
        if count.ge(&self.member_count) {
            return Err(crate::MultisigError::InvalidThreshold.into());
        }
        self.minimum_vote_count = count;
        Ok(())
    }

    #[inline(always)]
    pub fn set_minimum_member_count(&mut self, count: u32) -> Result<()> {
        if count.lt(&self.member_count) {
            return Err(crate::MultisigError::InvalidMemberCount.into());
        }
        self.minimum_member_count = count;
        Ok(())
    }
}

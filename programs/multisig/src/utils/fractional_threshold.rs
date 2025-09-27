use anchor_lang::prelude::*;

use crate::MultisigError;

// Stores a representation of a fractional value, used for representing thresholds
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct FractionalThreshold {
    numerator: u32,
    denominator: u32,
}

impl FractionalThreshold {
    pub fn is_valid(&self) -> Result<()> {
        if self.denominator.eq(&0) || self.numerator.ge(&self.denominator) || self.numerator.eq(&0){
            return Err(MultisigError::InvalidThreshold.into());
        }

        Ok(())
    }

    pub fn new_from_values(numerator: u32, denominator: u32) -> Result<FractionalThreshold> {
        if denominator.eq(&0) || numerator.ge(&denominator) || numerator.eq(&0) {
            return Err(MultisigError::InvalidThreshold.into());
        }

        Ok(FractionalThreshold {
            numerator,
            denominator,
        })
    }

    /// Compares two fractional vaules
    pub fn less_than_or_equal(&self, numerator: u64, denominator: u64) -> Result<bool> {
        if self.denominator == 0 
            || denominator == 0
        {
            return Err(ProgramError::ArithmeticOverflow.into());
        }

        // Check: numerator/denominator >= self.numerator/self.denominator
        Ok((numerator)
            .checked_mul(u64::from(self.denominator))
            .ok_or(ProgramError::ArithmeticOverflow)?
            .ge(&denominator
                .checked_mul(u64::from(self.numerator))
                .ok_or(ProgramError::ArithmeticOverflow)?))
    }

}

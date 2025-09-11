use anchor_lang::prelude::*;

use crate::MultisigError;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct FractionalThreshold {
    numerator: u32,
    denominator: u32,
}

impl FractionalThreshold {
    pub fn is_valid(&self) -> Result<()> {
        if self.denominator.eq(&0) || self.numerator.ge(&self.denominator) {
            return Err(MultisigError::InvalidThreshold.into());
        }

        Ok(())
    }

    pub fn new_from_values(numerator: u32, denominator: u32) -> Result<FractionalThreshold> {
        if denominator.eq(&0) || numerator.ge(&denominator) {
            return Err(MultisigError::InvalidThreshold.into());
        }

        Ok(FractionalThreshold {
            numerator,
            denominator,
        })
    }
    /// Returns true if the fraction (numerator/denominator) condition is satisfied.
    /// Example: if threshold is 2/3, then 2 out of 3 (≈66.6%) must be reached.
    pub fn greater_than_or_equal(&self, numerator: u64, denominator: u64) -> Result<bool> {
        if self.denominator == 0 // Not really necessary since the threshold can't have zero denominator
            || denominator == 0
        {
            return Err(ProgramError::ArithmeticOverflow.into()); // avoid divide-by-zero and meaningless thresholds
        }

        // Check: numerator/denominator >= self.numerator/self.denominator
        Ok((numerator)
            .checked_mul(u64::from(self.denominator))
            .ok_or(ProgramError::ArithmeticOverflow)?
            .ge(&denominator
                .checked_mul(u64::from(self.numerator))
                .ok_or(ProgramError::ArithmeticOverflow)?))
    }

    /// Example: if threshold is 2/3, then 2 out of 3 (≈66.6%) must be reached.
    pub fn threshold_greater_than_or_equal(&self, threshold: &FractionalThreshold) -> Result<bool> {
        // It's safe to use unwrap since the threshold can't be in an invalid state
        self.greater_than_or_equal(
            u64::from(threshold.numerator),
            u64::from(threshold.denominator),
        )
    }

    pub fn normalize_other(&self, threshold: &mut FractionalThreshold) -> Result<()> {
        if self.threshold_greater_than_or_equal(threshold)? {
            *threshold = FractionalThreshold {
                numerator: self.denominator.saturating_sub(self.numerator),
                denominator: self.denominator,
            };
        }

        Ok(())
    }
}

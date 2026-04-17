use anchor_lang::prelude::*;

use crate::MultisigError;

// Stores a representation of a fractional value, used for representing thresholds
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, Debug, PartialEq, Eq)]
pub struct FractionalThreshold {
    pub numerator: u32,
    pub denominator: u32,
}

#[cfg(feature = "test-helpers")]
impl FractionalThreshold {
    pub fn from_unchecked(numerator: u32, denominator: u32) -> FractionalThreshold {
        FractionalThreshold {
            numerator,
            denominator,
        }
    }
}

impl FractionalThreshold {
    pub fn is_valid(&self) -> Result<()> {
        if self.denominator.eq(&0) || self.numerator.gt(&self.denominator) || self.numerator.eq(&0)
        {
            return Err(MultisigError::InvalidThreshold.into());
        }

        Ok(())
    }

    pub fn new_from_values(numerator: u32, denominator: u32) -> Result<FractionalThreshold> {
        if denominator.eq(&0) || numerator.gt(&denominator) || numerator.eq(&0) {
            return Err(MultisigError::InvalidThreshold.into());
        }

        Ok(FractionalThreshold {
            numerator,
            denominator,
        })
    }

    pub fn validate_non_overlapping_pair(
        pass_threshold: FractionalThreshold,
        fail_threshold: FractionalThreshold,
    ) -> Result<()> {
        pass_threshold.is_valid()?;
        fail_threshold.is_valid()?;

        let pass_num = u128::from(pass_threshold.numerator);
        let pass_den = u128::from(pass_threshold.denominator);
        let fail_num = u128::from(fail_threshold.numerator);
        let fail_den = u128::from(fail_threshold.denominator);

        if pass_num * fail_den + fail_num * pass_den <= pass_den * fail_den {
            return Err(MultisigError::InvalidThreshold.into());
        }

        Ok(())
    }

    /// Compares two fractional vaules
    pub fn less_than_or_equal(&self, numerator: u64, denominator: u64) -> Result<bool> {
        if self.denominator == 0 || denominator == 0 {
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

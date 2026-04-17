use anchor_lang::prelude::*;

use crate::MultisigError;

/// Stores information about a group member
#[account]
#[derive(InitSpace)]
pub struct AssetMember {
    pub user: Pubkey,
    pub group: Pubkey,
    pub asset: Pubkey,
    pub weight: u32,
    pub permissions: Permissions,
    pub account_bump: u8,
}

/// Stores information about a group member
#[account]
#[derive(InitSpace)]
pub struct GroupMember {
    pub user: Pubkey,
    pub group: Pubkey,
    pub weight: u32,
    pub permissions: Permissions,
    pub account_bump: u8,
}

impl AssetMember {
    #[inline(always)]
    fn validate_weight(weight: u32, max_weight: u32) -> Result<()> {
        require_gt!(weight, 0, MultisigError::InvalidMemberWeight);
        require_gte!(max_weight, weight, MultisigError::InvalidMemberWeight);
        Ok(())
    }

    #[inline(always)]
    pub fn new(
        user: Pubkey,
        group: Pubkey,
        asset: Pubkey,
        permissions: Permissions,
        weight: u32,
        account_bump: u8,
        max_weight: u32,
    ) -> Result<Self> {
        permissions.is_valid()?;
        Self::validate_weight(weight, max_weight)?;

        Ok(Self {
            user,
            group,
            asset,
            permissions,
            weight,
            account_bump,
        })
    }

    #[inline(always)]
    pub fn set_weight(&mut self, weight: u32, max_weight: u32) -> Result<()> {
        Self::validate_weight(weight, max_weight)?;
        self.weight = weight;
        Ok(())
    }

    #[inline(always)]
    pub fn has_propose(&self) -> bool {
        self.permissions.has_propose()
    }

    #[inline(always)]
    pub fn set_propose(&mut self, enable: bool) {
        self.permissions.set_propose(enable);
    }

    #[inline(always)]
    pub fn has_add_asset(&self) -> bool {
        self.permissions.has_add_asset()
    }

    #[inline(always)]
    pub fn set_add_asset(&mut self, enable: bool) {
        self.permissions.set_add_asset(enable);
    }
}

impl GroupMember {
    #[inline(always)]
    fn validate_weight(weight: u32, max_weight: u32) -> Result<()> {
        require_gt!(weight, 0, MultisigError::InvalidMemberWeight);
        require_gte!(max_weight, weight, MultisigError::InvalidMemberWeight);
        Ok(())
    }

    #[inline(always)]
    pub fn new(
        user: Pubkey,
        group: Pubkey,
        permissions: Permissions,
        weight: u32,
        account_bump: u8,
        max_weight: u32,
    ) -> Result<Self> {
        permissions.is_valid()?;
        Self::validate_weight(weight, max_weight)?;

        Ok(Self {
            user,
            group,
            permissions,
            weight,
            account_bump,
        })
    }

    #[inline(always)]
    pub fn set_weight(&mut self, weight: u32, max_weight: u32) -> Result<()> {
        Self::validate_weight(weight, max_weight)?;
        self.weight = weight;
        Ok(())
    }

    #[inline(always)]
    pub fn has_propose(&self) -> bool {
        self.permissions.has_propose()
    }

    #[inline(always)]
    pub fn set_propose(&mut self, enable: bool) {
        self.permissions.set_propose(enable);
    }

    #[inline(always)]
    pub fn has_add_asset(&self) -> bool {
        self.permissions.has_add_asset()
    }

    #[inline(always)]
    pub fn set_add_asset(&mut self, enable: bool) {
        self.permissions.set_add_asset(enable);
    }
}

// Stores permissions with a bit flag
/// 1 << ? - Vote - Anyone with a weight > 0 can vote.
/// 1 << ? - Execute - If the proposal passed then anyone can execute the transaction,
/// but whatever rent they pay would not be returned to them but to the rent collector.
/// 1 << 0 - Propose
/// 1 << 1 - Add asset
#[derive(AnchorDeserialize, AnchorSerialize, InitSpace, Clone, Copy)]
pub struct Permissions {
    permissions: u8,
}

impl TryFrom<u8> for Permissions {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self> {
        let permissions = Permissions { permissions: value };

        permissions.is_valid()?;

        Ok(permissions)
    }
}

#[cfg(feature = "test-helpers")]
impl Permissions {
    pub fn from_unchecked(value: u8) -> Self {
        Permissions { permissions: value }
    }

    pub fn from_flags(propose: bool, add_asset: bool) -> Permissions {
        let mut flag = 0b00000000u8;

        if propose {
            flag |= 0b00000001u8;
        }

        if add_asset {
            flag |= 0b00000010u8;
        }

        Permissions { permissions: flag }
    }
}

impl Permissions {
    const VALID_STATE_MASK: u8 = 0b11111100;

    /// Check if "Propose" permission is set
    #[inline(always)]
    pub fn has_propose(&self) -> bool {
        (self.permissions & (1 << 0)) != 0
    }

    /// Set or unset "Propose" permission
    #[inline]
    pub fn set_propose(&mut self, enable: bool) {
        if enable {
            self.permissions |= 1 << 0;
        } else {
            self.permissions &= !(1 << 0);
        }
    }

    /// Check if "Add asset" permission is set
    #[inline(always)]
    pub fn has_add_asset(&self) -> bool {
        (self.permissions & (1 << 1)) != 0
    }

    /// Set or unset "Add asset" permission
    #[inline]
    pub fn set_add_asset(&mut self, enable: bool) {
        if enable {
            self.permissions |= 1 << 1;
        } else {
            self.permissions &= !(1 << 1);
        }
    }

    #[inline]
    pub fn is_valid(&self) -> Result<()> {
        if (self.permissions & Self::VALID_STATE_MASK).ne(&0) {
            return Err(MultisigError::InvalidPermissions.into());
        }

        Ok(())
    }
}

use anchor_lang::prelude::*;

use crate::MultisigError;

/// Stores information about a group member
#[account]
#[derive(InitSpace)]
pub struct AssetMember {
    user: Pubkey,
    group:Pubkey,
    asset: Pubkey,
    permissions: Permissions,
    weight: u32,
    account_bump: u8,
}

/// Stores information about a group member
#[account]
#[derive(InitSpace)]
pub struct GroupMember {
    user: Pubkey,
    group:Pubkey,    
    permissions: Permissions,
    weight: u32,
    account_bump: u8,
}

impl AssetMember {
    #[inline(always)]
    pub fn new(
        user: Pubkey,
        group:Pubkey,
        asset: Pubkey,
        permissions: Permissions,
        weight: u32,
        account_bump: u8,
        max_weight: u32,
    ) -> Result<Self> {
        permissions.validate()?;

        Ok(Self {
            user,
            group,
            asset,
            permissions,
            weight: weight.min(max_weight),
            account_bump,
        })
    }

    #[inline(always)]
    pub fn get_user(&self) -> &Pubkey {
        &self.user
    }

    pub fn get_group(&self) -> &Pubkey{
        &self.group
    }

    #[inline(always)]
    pub fn get_asset(&self) -> &Pubkey {
        &self.asset
    }

    #[inline(always)]
    pub fn get_permissions(&self) -> Permissions {
        self.permissions
    }

    #[inline(always)]
    pub fn get_weight(&self) -> u32 {
        self.weight
    }

    #[inline(always)]
    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
    }

    #[inline(always)]
    pub fn set_user(&mut self, user: Pubkey) {
        self.user = user;
    }

    #[inline(always)]
    pub fn set_asset(&mut self, asset: Pubkey) {
        self.asset = asset;
    }

    #[inline(always)]
    pub fn set_permissions(&mut self, permissions: Permissions) {
        self.permissions = permissions;
    }

    #[inline(always)]
    pub fn set_weight(&mut self, weight: u32, max_weight: u32) {
        self.weight = weight.min(max_weight);
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
    pub fn new(
        user: Pubkey,
        group: Pubkey,
        permissions: Permissions,
        weight: u32,
        account_bump: u8,
        max_weight: u32,
    ) -> Result<Self> {
        permissions.validate()?;

        Ok(Self {
            user,
            group,
            permissions,
            weight: weight.min(max_weight),
            account_bump,
        })
    }

    #[inline(always)]
    pub fn get_user(&self) -> &Pubkey {
        &self.user
    }

    pub fn get_group(&self) -> &Pubkey{
        &self.group
    }

    #[inline(always)]
    pub fn get_permissions(&self) -> Permissions {
        self.permissions
    }

    #[inline(always)]
    pub fn get_weight(&self) -> u32 {
        self.weight
    }

    #[inline(always)]
    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
    }

    #[inline(always)]
    pub fn set_user(&mut self, user: Pubkey) {
        self.user = user;
    }

    #[inline(always)]
    pub fn set_permissions(&mut self, permissions: Permissions) {
        self.permissions = permissions;
    }

    #[inline(always)]
    pub fn set_weight(&mut self, weight: u32, max_weight: u32) {
        self.weight = weight.min(max_weight);
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
    pub fn validate(&self) -> Result<()> {
        if (self.permissions & Self::VALID_STATE_MASK).ne(&0) {
            return Err(MultisigError::InvalidPermissions.into());
        }

        Ok(())
    }
}

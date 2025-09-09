use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct AssetMember {
    user: Pubkey,
    asset: Pubkey,
    permissions: Permissions,
    weight: u32,
    account_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct GroupMember {
    user: Pubkey,
    permissions: Permissions,
    weight: u32,
    account_bump: u8,
}

impl AssetMember {
    pub fn new(
        user: Pubkey,
        asset: Pubkey,
        permissions: Permissions,
        weight: u32,
        account_bump: u8,
        max_weight: u32,
    ) -> Result<Self> {
        
        Ok(Self {
            user,
            asset,
            permissions,
            weight:weight.min(max_weight),
            account_bump,
        })
    }

    pub fn get_user(&self) -> Pubkey {
        self.user
    }

    pub fn get_asset(&self) -> Pubkey {
        self.asset
    }

    pub fn get_permissions(&self) -> Permissions {
        self.permissions
    }

    pub fn get_weight(&self) -> u32 {
        self.weight
    }

    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
    }

    pub fn set_user(&mut self, user: Pubkey) {
        self.user = user;
    }

    pub fn set_asset(&mut self, asset: Pubkey) {
        self.asset = asset;
    }

    pub fn set_permissions(&mut self, permissions: Permissions) {
        self.permissions = permissions;
    }

    pub fn set_weight(&mut self, weight: u32, max_weight: u32){
        self.weight = weight.min(max_weight);
    }

    pub fn has_propose(&self) -> bool {
        self.permissions.has_propose()
    }

    pub fn set_propose(&mut self, enable: bool) {
        self.permissions.set_propose(enable);
    }

    pub fn has_add_asset(&self) -> bool {
        self.permissions.has_add_asset()
    }

    pub fn set_add_asset(&mut self, enable: bool) {
        self.permissions.set_add_asset(enable);
    }
}

impl GroupMember {
    pub fn new(
        user: Pubkey,
        permissions: Permissions,
        weight: u32,
        account_bump: u8,
        max_weight: u32,
    ) -> Result<Self> {
        
        Ok(Self {
            user,
            permissions,
            weight:weight.min(max_weight),
            account_bump,
        })
    }

    pub fn get_user(&self) -> &Pubkey {
        &self.user
    }

    pub fn get_permissions(&self) -> Permissions {
        self.permissions
    }

    pub fn get_weight(&self) -> u32 {
        self.weight
    }

    pub fn get_account_bump(&self) -> u8 {
        self.account_bump
    }

    pub fn set_user(&mut self, user: Pubkey) {
        self.user = user;
    }

    pub fn set_permissions(&mut self, permissions: Permissions) {
        self.permissions = permissions;
    }

    pub fn set_weight(&mut self, weight: u32, max_weight: u32){
        self.weight = weight.min(max_weight);
    }

    pub fn has_propose(&self) -> bool {
        self.permissions.has_propose()
    }

    pub fn set_propose(&mut self, enable: bool) {
        self.permissions.set_propose(enable);
    }

    pub fn has_add_asset(&self) -> bool {
        self.permissions.has_add_asset()
    }

    pub fn set_add_asset(&mut self, enable: bool) {
        self.permissions.set_add_asset(enable);
    }
}


#[derive(AnchorDeserialize, AnchorSerialize, InitSpace, Clone, Copy)]
pub struct Permissions {
    permissions: u8,
    // 1 << ? - Vote - Anyone with a weight > 0 can vote.
    // 1 << ? - Execute - If the proposal passed then anyone can execute the transaction.
    // 1 << 0 - Propose
    // 1 << 1 - Add asset
}

impl Permissions {
    /// Check if "Propose" permission is set
    pub fn has_propose(&self) -> bool {
        (self.permissions & (1 << 0)) != 0
    }

    /// Set or unset "Propose" permission
    pub fn set_propose(&mut self, enable: bool) {
        if enable {
            self.permissions |= 1 << 0;
        } else {
            self.permissions &= !(1 << 0);
        }
    }

    /// Check if "Add asset" permission is set
    pub fn has_add_asset(&self) -> bool {
        (self.permissions & (1 << 1)) != 0
    }

    /// Set or unset "Add asset" permission
    pub fn set_add_asset(&mut self, enable: bool) {
        if enable {
            self.permissions |= 1 << 1;
        } else {
            self.permissions &= !(1 << 1);
        }
    }
}
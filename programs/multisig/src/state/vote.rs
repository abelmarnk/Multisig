use anchor_lang::prelude::*;

// Stores information about a proposal vote
#[account]
#[derive(InitSpace)]
pub struct VoteRecord {
    pub voter: Pubkey,
    pub proposal: Pubkey,
    pub asset_index: Option<u8>,
    pub account_bump: u8,
    pub vote_choice: VoteChoice,
}

#[derive(AnchorDeserialize, AnchorSerialize, InitSpace, Clone, Copy, PartialEq)]
pub enum VoteChoice {
    For,
    Against,
}

impl VoteRecord {
    #[inline(always)]
    pub fn new(
        voter: Pubkey,
        proposal: Pubkey,
        asset_index: Option<u8>,
        account_bump: u8,
        vote_choice: VoteChoice,
    ) -> Self {
        Self {
            voter,
            proposal,
            asset_index,
            account_bump,
            vote_choice,
        }
    }

    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.voter != Pubkey::default()
    }
}

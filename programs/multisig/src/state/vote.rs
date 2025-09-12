use anchor_lang::prelude::*;

// Stores information about a proposal vote
#[account]
#[derive(InitSpace)]
pub struct VoteRecord {
    voter: Pubkey,
    proposal: Pubkey,
    asset_index: Option<u8>,
    account_bump: u8,
    vote_choice: VoteChoice,
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
    pub fn get_proposal(&self) -> &Pubkey {
         &self.proposal 
    }

    #[inline(always)]
    pub fn get_voter(&self) -> &Pubkey { 
        &self.voter 
    }

    #[inline(always)]
    pub fn get_asset_index(&self) -> Option<u8> { 
        self.asset_index 
    }

    #[inline(always)]
    pub fn get_account_bump(&self) -> u8 {
         self.account_bump 
    }

    #[inline(always)]
    pub fn get_vote_choice(&self) -> VoteChoice { 
        self.vote_choice 
    }

    #[inline(always)]
    pub fn set_vote_choice(&mut self, choice: VoteChoice) { 
        self.vote_choice = choice; 
    }
}

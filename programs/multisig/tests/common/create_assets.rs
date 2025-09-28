use litesvm::LiteSVM;
use solana_sdk::{
    account::Account, program_option::COption, program_pack::Pack, 
    pubkey::Pubkey, signature::Keypair, signer::Signer
};
use spl_token::{
    ID as TOKEN_PROGRAM_ID, state::{
        Account as TokenAccount, AccountState, Mint
    }
};
use spl_associated_token_account::{
    get_associated_token_address
};
use anyhow::Result;

/// Create a mint into the svm with the keypair, owner and mint & freeze authority
pub fn create_mint(svm:&mut LiteSVM, maybe_mint_authority_key:COption<&Pubkey>, 
        maybe_freeze_authority_key:COption<&Pubkey>, is_initialized:bool)->Result<Keypair>{
    let mint_keypair = Keypair::new();

    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);

    let mint = Mint{
        mint_authority:maybe_mint_authority_key.cloned(),
        supply:100_000_000_000,
        decimals:9,
        is_initialized,
        freeze_authority:maybe_freeze_authority_key.cloned()
    };

    let mut mint_data = Vec::with_capacity(Mint::LEN);

    Mint::pack(mint, &mut mint_data).unwrap();

    let mint_account = Account{
            lamports:rent,
            data:mint_data,
            owner:TOKEN_PROGRAM_ID,
            executable:false,
            rent_epoch:0
    };

    svm.set_account(mint_keypair.pubkey(), mint_account)?;

    Ok(mint_keypair)
}

/// Create a token account into the svm with the mint, owner, state and authorities
pub fn create_token_account(svm:&mut LiteSVM, mint:&Pubkey, owner:&Pubkey, 
    delegate:COption<&Pubkey>, state:AccountState, close_authority:COption<&Pubkey>)->Result<Pubkey>{
    let token_account_key = get_associated_token_address(
            owner, mint);

    let rent = svm.minimum_balance_for_rent_exemption(TokenAccount::LEN);

    let token_account = TokenAccount{
        mint:*mint,
        owner:*owner,
        amount:1_000_000_000_000,
        delegate:delegate.cloned(),
        state,
        is_native:COption::None,
        delegated_amount:delegate.map(|_| 50_000_000_000).unwrap_or(0),
        close_authority:close_authority.cloned()
    };

    let mut token_account_data = Vec::with_capacity(TokenAccount::LEN);

    TokenAccount::pack(token_account, &mut token_account_data).unwrap();

    let token_account_account = Account{
            lamports:rent,
            data:token_account_data,
            owner:TOKEN_PROGRAM_ID,
            executable:false,
            rent_epoch:0
    };

    svm.set_account(token_account_key, token_account_account)?;

    Ok(token_account_key)
}

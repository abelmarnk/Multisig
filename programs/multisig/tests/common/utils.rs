#[cfg(feature = "test-helpers")]
use litesvm::LiteSVM;
use rand::{Rng};
use solana_sdk::{
    program_option::COption, 
    pubkey::{
        Pubkey,
    }
};
use multisig::{
    FractionalThreshold, ID as MULTISIG_PROGRAM_ID, Permissions
};
use anyhow::{
    Result
};
use std::path::{
    Path
};


#[inline(always)]
pub fn option_to_c_option<T>(option:Option<T>)->COption<T>{
    option.map_or(COption::None, |value|COption::Some(value))
}

#[inline(always)]
pub fn get_group(seed:&Pubkey)->Pubkey{
    return Pubkey::find_program_address(
        &[b"group", seed.as_ref()], &MULTISIG_PROGRAM_ID).0
}

#[inline(always)]
pub fn get_group_member(group:&Pubkey, member:&Pubkey)->Pubkey{
    return Pubkey::find_program_address(
        &[b"member", group.as_ref(), member.as_ref()], &MULTISIG_PROGRAM_ID).0
}

#[inline(always)]
pub fn add_multisig_program(svm:&mut LiteSVM)->Result<()>{
    let path = Path::new("target/deploy/multisig.so");

    svm.add_program_from_file(MULTISIG_PROGRAM_ID, &path)?;

    Ok(())
}

pub fn get_invalid_threshold(rng:&mut rand::rngs::ThreadRng)->FractionalThreshold{

    let value = rng.random::<u8>() % 4;

    match value {
        0 => FractionalThreshold::from_unchecked(1, 0),
        1 => FractionalThreshold::from_unchecked(0, 1),
        2 => FractionalThreshold::from_unchecked(1, 1),
        _ => {
                let numerator = rng.random::<u32>();

                let mut denominator = rng.random::<u32>();

                while denominator.lt(&numerator){
                    denominator = rng.random::<u32>();
                }

                FractionalThreshold::from_unchecked(numerator, denominator)},
    }
}


pub fn get_invalid_permissions(rng:&mut rand::rngs::ThreadRng)->Permissions{
    
    let value = rng.random::<u8>() % 4;

    Permissions::from_unchecked(
        match value {
            0 => 0b00000011u8 | 0b00000100u8,
            1 => 0b00000011u8 | 0b00001000u8,
            2 => 0b00000011u8 | 0b00010000u8,
            _ => 0b10000000u8 | rng.random::<u8>()
        }
    )
}
#![cfg(feature = "test-helpers")]
use std::array;

use anchor_lang::{
    InstructionData
};
use litesvm::{
    LiteSVM
};
use multisig::{
    FractionalThreshold, ID as MULTISIG_PROGRAM_ID, 
    Permissions, instruction::CreateGroup, 
    instructions::CreateGroupInstructionArgs
};
use anyhow::{
    Result
};
use rand::{Rng, rng};
use solana_sdk::{
    instruction::{
        Instruction,
        AccountMeta
    }, 
    pubkey::Pubkey, 
    signer::{
        Signer, keypair::Keypair
    }, system_program::ID as SYSTEM_PROGRAM_ID, 
    transaction::Transaction
};

mod common;
use common::{
    utils::{
        add_multisig_program
    }
};

use crate::common::{get_group, get_group_member, get_invalid_permissions, get_invalid_threshold};

// The create group instruction is basic it requires that the
// permissions, thresholds & counts provided to it are valid
#[test]
fn test_passing(){
    let mut svm = LiteSVM::new();

    add_multisig_program_with_log(&mut svm);

    let result = 
        TestSetup::with_default(&mut svm);
    
    let (instructions, payer_keypair) = match result {
        Ok(result) => result,
        Err(error) => {
            println!("Failed to create instruction....\n\n");
            panic!("Error: {}", error.to_string());
        }
    };

    let payer_key = payer_keypair.pubkey();

    let recent_blockhash = svm.latest_blockhash();

    let transaction = Transaction::new_signed_with_payer(
        &instructions, Some(&payer_key), &[payer_keypair], 
        recent_blockhash
    );

    let result = svm.send_transaction(transaction);

    match result {
        Ok(result)=>{
            println!("Program succeded....\n\n");
            print!("CU consumed: {:?}", result.compute_units_consumed);
        },
        Err(error)=>{
            println!("Program failed....\n\n");
            panic!("Failed transaction metadata: {:?}", error);
        }
    }
}

#[test]
fn test_fails_with_invalid_threshold(){
    let mut svm = LiteSVM::new();

    add_multisig_program_with_log(&mut svm);

    let mut test_setup = TestSetup::new();

    let result = 
        test_setup.with_invalid_threshold(&mut svm);
    
    let (instructions, payer_keypair) = match result {
        Ok(result) => result,
        Err(error) => {
            println!("Failed to create instruction....\n\n");
            panic!("Error: {}", error.to_string());
        }
    };

    let payer_key = payer_keypair.pubkey();

    let recent_blockhash = svm.latest_blockhash();

    let transaction = Transaction::new_signed_with_payer(
        &instructions, Some(&payer_key), &[payer_keypair], 
        recent_blockhash
    );

    let result = svm.send_transaction(transaction);

    match result {
        Ok(_)=>{
            println!("Program succeded....\n\n");
            panic!("Wait!!!, Program should have failed due to invalid threshold");
        },
        Err(error)=>{
            println!("Program failed....\n\n");
            println!("Failed transaction error: {:?}", error.err);
        }
    }
}

#[test]
fn test_fails_with_invalid_permissions(){
    let mut svm = LiteSVM::new();

    add_multisig_program_with_log(&mut svm);

    let mut test_setup = TestSetup::new();

    let result = 
        test_setup.with_invalid_permissions(&mut svm);
    
    let (instructions, payer_keypair) = match result {
        Ok(result) => result,
        Err(error) => {
            println!("Failed to create instruction....\n\n");
            panic!("Error: {}", error.to_string());
        }
    };

    let payer_key = payer_keypair.pubkey();

    let recent_blockhash = svm.latest_blockhash();

    let transaction = Transaction::new_signed_with_payer(
        &instructions, Some(&payer_key), &[payer_keypair], 
        recent_blockhash
    );

    let result = svm.send_transaction(transaction);

    match result {
        Ok(_)=>{
            println!("Program succeded....\n\n");
            panic!("Wait!!!, Program should have failed due to invalid permissions");
        },
        Err(error)=>{
            println!("Program failed....\n\n");
            println!("Failed transaction error: {:?}", error.err);
        }
    }
}

#[test]
fn test_fails_with_invalid_minimum_member_count(){
    let mut svm = LiteSVM::new();

    add_multisig_program_with_log(&mut svm);

    let mut test_setup = TestSetup::new();

    let result = 
        test_setup.with_invalid_minimum_member_count(&mut svm);
    
    let (instructions, payer_keypair) = match result {
        Ok(result) => result,
        Err(error) => {
            println!("Failed to create instruction....\n\n");
            panic!("Error: {}", error.to_string());
        }
    };

    let payer_key = payer_keypair.pubkey();

    let recent_blockhash = svm.latest_blockhash();

    let transaction = Transaction::new_signed_with_payer(
        &instructions, Some(&payer_key), &[payer_keypair], 
        recent_blockhash
    );

    let result = svm.send_transaction(transaction);

    match result {
        Ok(_)=>{
            println!("Program succeded....\n\n");
            panic!("Wait!!!, Program should have failed due to invalid minimum member count");
        },
        Err(error)=>{
            println!("Program failed....\n\n");
            println!("Failed transaction error: {:?}", error.err);
        }
    }
}

#[test]
fn test_fails_with_invalid_minimum_vote_count(){
    let mut svm = LiteSVM::new();

    add_multisig_program_with_log(&mut svm);

    let mut test_setup = TestSetup::new();

    let result = 
        test_setup.with_invalid_minimum_vote_count(&mut svm);
    
    let (instructions, payer_keypair) = match result {
        Ok(result) => result,
        Err(error) => {
            println!("Failed to create instruction....\n\n");
            panic!("Error: {}", error.to_string());
        }
    };

    let payer_key = payer_keypair.pubkey();

    let recent_blockhash = svm.latest_blockhash();

    let transaction = Transaction::new_signed_with_payer(
        &instructions, Some(&payer_key), &[payer_keypair], 
        recent_blockhash
    );

    let result = svm.send_transaction(transaction);

    match result {
        Ok(_)=>{
            println!("Program succeded....\n\n");
            panic!("Wait!!!, Program should have failed due to invalid minimum vote count");
        },
        Err(error)=>{
            println!("Program failed....\n\n");
            println!("Failed transaction error: {:?}", error.err);
        }
    }
}

struct TestSetup{
    rng:rand::rngs::ThreadRng
}

impl TestSetup{
    const SYSTEM_PROGRAM_ID:Pubkey = SYSTEM_PROGRAM_ID;
    const MULTISIG_PROGRAM_ID:Pubkey = MULTISIG_PROGRAM_ID;

    pub fn new()->Self{
        TestSetup { rng: rng() }
    }

    /// The default case — it passes.
    pub fn with_default(svm: &mut LiteSVM) -> Result<([Instruction;1], Keypair)> {
        let positive_threshold = FractionalThreshold::new_from_values(1, 2).unwrap();
        let negative_threshold = FractionalThreshold::new_from_values(1, 3).unwrap();

        let create_group_instruction_args = CreateGroupInstructionArgs {
            group_seed: Pubkey::new_unique(),
            rent_collector: Pubkey::new_unique(),

            add_threshold: positive_threshold,
            not_add_threshold: negative_threshold,
            remove_threshold: positive_threshold,
            not_remove_threshold: negative_threshold,
            change_config_threshold: positive_threshold,
            not_change_config_threshold: negative_threshold,

            max_member_weight: 100,
            minimum_member_count: 5,
            minimum_vote_count: 3,
            member_weights: [20; 5],
            member_permissions: [Permissions::from_flags(true, true); 5],
        };

        Self::builder(svm, create_group_instruction_args)
    }

    pub fn builder(svm:&mut LiteSVM, create_group_instruction_args:CreateGroupInstructionArgs)->Result<([Instruction;1], Keypair)>{

        // Add the payer into the svm
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

        let members: [Pubkey; 5] = array::from_fn(|_| Pubkey::new_unique());

        let group = get_group(&create_group_instruction_args.group_seed);

        let create_group_instruction_accounts:Vec<AccountMeta> = 
            vec![
                AccountMeta::new(group, false),
                AccountMeta::new_readonly(members[0], false),
                AccountMeta::new_readonly(members[1], false),
                AccountMeta::new_readonly(members[2], false),
                AccountMeta::new_readonly(members[3], false),
                AccountMeta::new_readonly(members[4], false),
                AccountMeta::new(get_group_member(&group, &members[0]), false),
                AccountMeta::new(get_group_member(&group, &members[1]), false),
                AccountMeta::new(get_group_member(&group, &members[2]), false),
                AccountMeta::new(get_group_member(&group, &members[3]), false),
                AccountMeta::new(get_group_member(&group, &members[4]), false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(Self::SYSTEM_PROGRAM_ID, false)
            ];

        let args = CreateGroup{
            args:create_group_instruction_args
        };

        let create_group_instruction = Instruction{
            program_id:Self::MULTISIG_PROGRAM_ID,
            accounts:create_group_instruction_accounts,
            data:args.data()
        };

        Ok(([create_group_instruction], payer))
    }

    fn with_invalid_threshold(&mut self, svm: &mut LiteSVM) -> Result<([Instruction;1], Keypair)> {
        // Build the instruction args with *invalid thresholds*
        let create_group_instruction_args = CreateGroupInstructionArgs {
            group_seed: Pubkey::new_unique(),
            rent_collector: Pubkey::new_unique(),

            add_threshold: get_invalid_threshold(&mut self.rng),
            not_add_threshold: get_invalid_threshold(&mut self.rng),
            remove_threshold: get_invalid_threshold(&mut self.rng),
            not_remove_threshold: get_invalid_threshold(&mut self.rng),
            change_config_threshold: get_invalid_threshold(&mut self.rng),
            not_change_config_threshold: get_invalid_threshold(&mut self.rng),

            max_member_weight: 100,
            minimum_member_count: 5,
            minimum_vote_count: 3,
            member_weights: [20; 5],
            member_permissions: [Permissions::from_flags(true, true); 5],
        };

        Self::builder(svm, create_group_instruction_args)
    }

    fn with_invalid_permissions(&mut self, svm:&mut LiteSVM) -> Result<([Instruction;1], Keypair)>{
        let member_permissions:[Permissions; 5] = 
            array::from_fn(|_| get_invalid_permissions(&mut self.rng));

        let positive_threshold = 
            FractionalThreshold::new_from_values(1, 2).unwrap();
        let negative_threshold = 
            FractionalThreshold::new_from_values(1, 3).unwrap();

        let create_group_instruction_args = CreateGroupInstructionArgs {
            group_seed: Pubkey::new_unique(),
            rent_collector: Pubkey::new_unique(),

            add_threshold: positive_threshold,
            not_add_threshold: negative_threshold,
            remove_threshold: positive_threshold,
            not_remove_threshold: negative_threshold,
            change_config_threshold: positive_threshold,
            not_change_config_threshold: negative_threshold,

            max_member_weight: 100,
            minimum_member_count: 5,
            minimum_vote_count: 3,
            member_weights: [20; 5],
            member_permissions
        };

        Self::builder(svm, create_group_instruction_args)


    }

     fn with_invalid_minimum_member_count(&mut self, svm: &mut LiteSVM) -> Result<([Instruction;1], Keypair)> {
        let positive_threshold = FractionalThreshold::new_from_values(1, 2).unwrap();
        let negative_threshold = FractionalThreshold::new_from_values(1, 3).unwrap();

        // Generate a random invalid minimum member count (> 5)
        let invalid_min_member_count = self.rng.random::<u32>();

        let create_group_instruction_args = CreateGroupInstructionArgs {
            group_seed: Pubkey::new_unique(),
            rent_collector: Pubkey::new_unique(),

            add_threshold: positive_threshold,
            not_add_threshold: negative_threshold,
            remove_threshold: positive_threshold,
            not_remove_threshold: negative_threshold,
            change_config_threshold: positive_threshold,
            not_change_config_threshold: negative_threshold,

            max_member_weight: 100,
            minimum_member_count: invalid_min_member_count,
            minimum_vote_count: 3,
            member_weights: [20; 5],
            member_permissions: [Permissions::from_flags(true, true); 5],
        };

        Self::builder(svm, create_group_instruction_args)
    }

    fn with_invalid_minimum_vote_count(&mut self, svm: &mut LiteSVM) -> Result<([Instruction;1], Keypair)> {
        let positive_threshold = FractionalThreshold::new_from_values(1, 2).unwrap();
        let negative_threshold = FractionalThreshold::new_from_values(1, 3).unwrap();

        // Generate a random invalid minimum vote count (≥ 5)
        let invalid_min_vote_count = self.rng.random_range(5..=20);

        let create_group_instruction_args = CreateGroupInstructionArgs {
            group_seed: Pubkey::new_unique(),
            rent_collector: Pubkey::new_unique(),

            add_threshold: positive_threshold,
            not_add_threshold: negative_threshold,
            remove_threshold: positive_threshold,
            not_remove_threshold: negative_threshold,
            change_config_threshold: positive_threshold,
            not_change_config_threshold: negative_threshold,

            max_member_weight: 100,
            minimum_member_count: 5,
            minimum_vote_count: invalid_min_vote_count,
            member_weights: [20; 5],
            member_permissions: [Permissions::from_flags(true, true); 5],
        };

        Self::builder(svm, create_group_instruction_args)
    }

}

fn add_multisig_program_with_log(svm: &mut LiteSVM){
    // Add the test program
    let error = add_multisig_program(svm);

    match error {
        Ok(()) => {},
        Err(error)=>{
            println!("Failed to add multisig program....\n\n");
            panic!("Error: {}", error.to_string());
        }
    }
}
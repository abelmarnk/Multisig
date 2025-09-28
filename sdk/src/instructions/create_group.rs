use anchor_lang::{
    InstructionData, prelude::*, 
    solana_program::instruction::Instruction,
    system_program::ID as SYSTEM_PROGRAM_ID
};
use multisig::{
    instruction::CreateGroup, 
    ID as MULTISIG_PROGRAM_ID
};

pub use multisig::instructions::CreateGroupInstructionArgs;

#[inline(always)]
fn get_group(seed:&Pubkey)->Pubkey{
    return Pubkey::find_program_address(
        &[b"group", seed.as_ref()], &MULTISIG_PROGRAM_ID).0
}

#[inline(always)]
fn get_group_member(group:&Pubkey, member:&Pubkey)->Pubkey{
    return Pubkey::find_program_address(
        &[b"member", group.as_ref(), member.as_ref()], &MULTISIG_PROGRAM_ID).0
}


pub fn create_group(args:CreateGroupInstructionArgs, 
        members:[Pubkey;5], payer:Pubkey)->Instruction{

        let group = get_group(&args.group_seed);

        let accounts = vec![
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
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
            ];

        let data = CreateGroup{
            args
        }.data();

        Instruction{
            program_id:MULTISIG_PROGRAM_ID,
            accounts,
            data
        }
}
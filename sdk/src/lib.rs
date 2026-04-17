use anchor_lang::{solana_program::hash, AnchorSerialize, InstructionData};
use multisig::{
    instruction as ix_data,
    instructions::{
        AddAssetMemberInstructionArgs, AddAssetMintInstructionArgs, AddAssetTokenInstructionArgs,
        AddGroupMemberInstructionArgs, AddMemberInResetModeArgs,
        CloseNormalVoteRecordInstructionArgs, CreateConfigProposalInstructionArgs,
        CreateEmergencyResetProposalArgs, CreateGroupInstructionArgs,
        CreateNormalProposalInstructionArgs, CreateProposalTransactionInstructionArgs,
        ExitPauseModeArgs, VoteOnConfigProposalInstructionArgs, VoteOnEmergencyResetArgs,
        VoteOnNormalProposalInstructionArgs,
    },
    SerializableInstruction,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

pub use multisig::{self, state, utils};

pub const PROGRAM_ID: Pubkey = multisig::ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pda {
    pub address: Pubkey,
    pub bump: u8,
}

fn pda(seeds: &[&[u8]]) -> Pda {
    let (address, bump) = Pubkey::find_program_address(seeds, &PROGRAM_ID);
    Pda { address, bump }
}

pub fn group_pda(group_seed: &Pubkey) -> Pda {
    pda(&[b"group", group_seed.as_ref()])
}

pub fn group_member_pda(group: &Pubkey, member: &Pubkey) -> Pda {
    pda(&[b"member", group.as_ref(), member.as_ref()])
}

pub fn asset_pda(group: &Pubkey, asset_address: &Pubkey) -> Pda {
    pda(&[b"asset", group.as_ref(), asset_address.as_ref()])
}

pub fn asset_authority_pda(group: &Pubkey, asset_address: &Pubkey) -> Pda {
    pda(&[b"authority", group.as_ref(), asset_address.as_ref()])
}

pub fn asset_member_pda(group: &Pubkey, asset_address: &Pubkey, member: &Pubkey) -> Pda {
    pda(&[
        b"asset-member",
        group.as_ref(),
        asset_address.as_ref(),
        member.as_ref(),
    ])
}

pub fn proposal_pda(group: &Pubkey, proposal_seed: &Pubkey) -> Pda {
    pda(&[b"proposal", group.as_ref(), proposal_seed.as_ref()])
}

pub fn proposal_transaction_pda(proposal: &Pubkey) -> Pda {
    pda(&[b"proposal-transaction", proposal.as_ref()])
}

pub fn normal_vote_record_pda(
    group: &Pubkey,
    proposal: &Pubkey,
    voter: &Pubkey,
    asset_index: u8,
) -> Pda {
    pda(&[
        b"vote-record",
        group.as_ref(),
        proposal.as_ref(),
        voter.as_ref(),
        &[asset_index],
    ])
}

pub fn config_vote_record_pda(group: &Pubkey, proposal: &Pubkey, voter: &Pubkey) -> Pda {
    pda(&[
        b"vote-record",
        group.as_ref(),
        proposal.as_ref(),
        voter.as_ref(),
    ])
}

pub fn serializable_instruction_bytes(
    instruction: &SerializableInstruction,
) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();
    instruction.serialize(&mut data)?;
    Ok(data)
}

pub fn serializable_instruction_hash(
    instruction: &SerializableInstruction,
) -> std::io::Result<[u8; hash::HASH_BYTES]> {
    Ok(hash::hash(&serializable_instruction_bytes(instruction)?).to_bytes())
}

/// Returns one hash per instruction - pass the result directly as `instruction_hashes`
/// when building a [`CreateNormalProposalInstructionArgs`].
pub fn serializable_instruction_hashes(
    instructions: &[SerializableInstruction],
) -> std::io::Result<Vec<[u8; hash::HASH_BYTES]>> {
    instructions
        .iter()
        .map(serializable_instruction_hash)
        .collect()
}

/// Serialises every instruction and returns the raw byte vectors - pass the result
/// directly as `raw_instructions` when building a [`CreateProposalTransactionInstructionArgs`].
pub fn serializable_instructions_bytes(
    instructions: &[SerializableInstruction],
) -> std::io::Result<Vec<Vec<u8>>> {
    instructions
        .iter()
        .map(serializable_instruction_bytes)
        .collect()
}

fn readonly(key: Pubkey) -> AccountMeta {
    AccountMeta::new_readonly(key, false)
}

fn writable(key: Pubkey) -> AccountMeta {
    AccountMeta::new(key, false)
}

fn signer(key: Pubkey) -> AccountMeta {
    AccountMeta::new(key, true)
}

fn readonly_signer(key: Pubkey) -> AccountMeta {
    AccountMeta::new_readonly(key, true)
}

fn optional_account(key: Option<Pubkey>, writable: bool) -> AccountMeta {
    match (key, writable) {
        (Some(key), true) => AccountMeta::new(key, false),
        (Some(key), false) => AccountMeta::new_readonly(key, false),
        (None, true) => AccountMeta::new_readonly(multisig::ID, false),
        (None, false) => AccountMeta::new_readonly(multisig::ID, false),
    }
}

pub fn create_group(
    args: CreateGroupInstructionArgs,
    payer: Pubkey,
    members: [Pubkey; 5],
) -> Instruction {
    let group = group_pda(&args.group_seed).address;
    let mut accounts = vec![
        writable(group),
        readonly(members[0]),
        readonly(members[1]),
        readonly(members[2]),
        readonly(members[3]),
        readonly(members[4]),
    ];

    accounts.extend(
        members
            .iter()
            .map(|member| writable(group_member_pda(&group, member).address)),
    );
    accounts.push(signer(payer));
    accounts.push(readonly(system_program::ID));

    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data: ix_data::CreateGroup { args }.data(),
    }
}

pub fn add_asset_mint(
    args: AddAssetMintInstructionArgs,
    payer: Pubkey,
    group: Pubkey,
    mint: Pubkey,
    token_program: Pubkey,
) -> Instruction {
    let asset = asset_pda(&group, &mint).address;
    let authority = asset_authority_pda(&group, &mint).address;
    let member_keys = [args.member_key_1, args.member_key_2, args.member_key_3];
    let mut accounts = vec![
        signer(payer),
        writable(group),
        readonly(mint),
        writable(asset),
        readonly(authority),
        readonly(group_member_pda(&group, &payer).address),
    ];

    accounts.extend(
        member_keys
            .iter()
            .map(|member| readonly(group_member_pda(&group, member).address)),
    );
    accounts.extend(
        member_keys
            .iter()
            .map(|member| writable(asset_member_pda(&group, &mint, member).address)),
    );
    accounts.push(readonly(token_program));
    accounts.push(readonly(system_program::ID));

    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data: ix_data::AddAssetMint { args }.data(),
    }
}

pub fn add_asset_token(
    args: AddAssetTokenInstructionArgs,
    payer: Pubkey,
    group: Pubkey,
    token: Pubkey,
    token_program: Pubkey,
) -> Instruction {
    let asset = asset_pda(&group, &token).address;
    let authority = asset_authority_pda(&group, &token).address;
    let member_keys = [args.member_key_1, args.member_key_2, args.member_key_3];
    let mut accounts = vec![
        signer(payer),
        writable(group),
        readonly(token),
        readonly(group_member_pda(&group, &payer).address),
    ];

    accounts.extend(
        member_keys
            .iter()
            .map(|member| readonly(group_member_pda(&group, member).address)),
    );
    accounts.push(writable(asset));
    accounts.push(readonly(authority));
    accounts.extend(
        member_keys
            .iter()
            .map(|member| writable(asset_member_pda(&group, &token, member).address)),
    );
    accounts.push(readonly(token_program));
    accounts.push(readonly(system_program::ID));

    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data: ix_data::AddAssetToken { args }.data(),
    }
}

pub fn create_normal_proposal(
    args: CreateNormalProposalInstructionArgs,
    group: Pubkey,
    proposer: Pubkey,
) -> Instruction {
    let proposal = proposal_pda(&group, &args.proposal_seed).address;

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            signer(proposer),
            readonly(group_member_pda(&group, &proposer).address),
            writable(proposal),
            readonly(system_program::ID),
        ],
        data: ix_data::CreateNormalProposal { args }.data(),
    }
}

pub fn create_config_proposal(
    args: CreateConfigProposalInstructionArgs,
    group: Pubkey,
    proposer: Pubkey,
    asset_address: Option<Pubkey>,
) -> Instruction {
    let proposal = proposal_pda(&group, &args.proposal_seed).address;
    let asset = asset_address.map(|asset_address| asset_pda(&group, &asset_address).address);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            signer(proposer),
            writable(group),
            optional_account(asset, true),
            readonly(group_member_pda(&group, &proposer).address),
            writable(proposal),
            readonly(system_program::ID),
        ],
        data: ix_data::CreateConfigProposal { args }.data(),
    }
}

pub fn create_proposal_transaction(
    args: CreateProposalTransactionInstructionArgs,
    group: Pubkey,
    proposal_seed: Pubkey,
    payer: Pubkey,
    asset_addresses: &[Pubkey],
) -> Instruction {
    let proposal = proposal_pda(&group, &proposal_seed).address;

    let mut accounts = vec![
        readonly(group),
        readonly(proposal),
        writable(proposal_transaction_pda(&proposal).address),
        signer(payer),
        readonly(system_program::ID),
    ];
    accounts.extend(
        asset_addresses
            .iter()
            .map(|asset_address| readonly(asset_pda(&group, asset_address).address)),
    );

    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data: ix_data::CreateProposalTransaction { args }.data(),
    }
}

pub fn add_group_member(
    args: AddGroupMemberInstructionArgs,
    group: Pubkey,
    proposal: Pubkey,
    proposer: Pubkey,
    payer: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(proposal),
            writable(proposer),
            writable(group_member_pda(&group, &args.new_member).address),
            signer(payer),
            readonly(system_program::ID),
        ],
        data: ix_data::AddGroupMember { args }.data(),
    }
}

pub fn add_asset_member(
    args: AddAssetMemberInstructionArgs,
    group: Pubkey,
    asset_address: Pubkey,
    proposal: Pubkey,
    proposer: Pubkey,
    payer: Pubkey,
) -> Instruction {
    let asset = asset_pda(&group, &asset_address).address;

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(asset),
            writable(proposal),
            writable(proposer),
            readonly(group_member_pda(&group, &args.new_member).address),
            writable(asset_member_pda(&group, &asset_address, &args.new_member).address),
            signer(payer),
            readonly(system_program::ID),
        ],
        data: ix_data::AddAssetMember { args }.data(),
    }
}

pub fn remove_group_member(
    group: Pubkey,
    member: Pubkey,
    proposal: Pubkey,
    rent_collector: Pubkey,
    proposer: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(group_member_pda(&group, &member).address),
            writable(proposal),
            writable(rent_collector),
            writable(proposer),
            readonly(system_program::ID),
        ],
        data: ix_data::RemoveGroupMember {}.data(),
    }
}

pub fn remove_asset_member(
    group: Pubkey,
    asset_address: Pubkey,
    member: Pubkey,
    proposal: Pubkey,
    rent_collector: Pubkey,
    proposer: Pubkey,
) -> Instruction {
    let asset = asset_pda(&group, &asset_address).address;

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(asset),
            writable(asset_member_pda(&group, &asset_address, &member).address),
            writable(proposal),
            writable(rent_collector),
            writable(proposer),
            readonly(system_program::ID),
        ],
        data: ix_data::RemoveAssetMember {}.data(),
    }
}

pub fn change_group_config(group: Pubkey, proposal: Pubkey, proposer: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![writable(group), writable(proposal), writable(proposer)],
        data: ix_data::ChangeGroupConfig {}.data(),
    }
}

pub fn change_asset_config(
    group: Pubkey,
    asset_address: Pubkey,
    proposal: Pubkey,
    proposer: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(asset_pda(&group, &asset_address).address),
            writable(proposal),
            writable(proposer),
        ],
        data: ix_data::ChangeAssetConfig {}.data(),
    }
}

pub fn vote_on_normal_proposal(
    args: VoteOnNormalProposalInstructionArgs,
    group: Pubkey,
    proposal: Pubkey,
    asset_address: Pubkey,
    voter: Pubkey,
) -> Instruction {
    let asset = asset_pda(&group, &asset_address).address;
    let proposal_transaction = proposal_transaction_pda(&proposal).address;

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(proposal),
            readonly(proposal_transaction),
            writable(asset),
            writable(group_member_pda(&group, &voter).address),
            writable(asset_member_pda(&group, &asset_address, &voter).address),
            writable(
                normal_vote_record_pda(&group, &proposal, &voter, args.voting_asset_index).address,
            ),
            signer(voter),
            readonly(system_program::ID),
        ],
        data: ix_data::VoteOnNormalProposal { args }.data(),
    }
}

pub fn vote_on_config_proposal(
    args: VoteOnConfigProposalInstructionArgs,
    group: Pubkey,
    proposal: Pubkey,
    voter: Pubkey,
    asset_address: Option<Pubkey>,
) -> Instruction {
    let asset = asset_address.map(|asset_address| asset_pda(&group, &asset_address).address);
    let asset_member =
        asset_address.map(|asset_address| asset_member_pda(&group, &asset_address, &voter).address);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(proposal),
            optional_account(asset, true),
            writable(group_member_pda(&group, &voter).address),
            optional_account(asset_member, true),
            writable(config_vote_record_pda(&group, &proposal, &voter).address),
            signer(voter),
            readonly(system_program::ID),
        ],
        data: ix_data::VoteOnConfigProposal { args }.data(),
    }
}

pub fn execute_proposal_transaction(
    group: Pubkey,
    proposal: Pubkey,
    proposal_transaction: Pubkey,
    rent_collector: Pubkey,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut accounts = vec![
        writable(proposal),
        writable(proposal_transaction),
        writable(group),
        writable(rent_collector),
    ];
    accounts.extend(remaining_accounts);

    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data: ix_data::ExecuteProposalTransaction {}.data(),
    }
}

pub fn close_proposal_transaction(
    group: Pubkey,
    proposal: Pubkey,
    proposal_transaction: Pubkey,
    rent_collector: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            readonly(group),
            readonly(proposal),
            writable(proposal_transaction),
            writable(rent_collector),
        ],
        data: ix_data::CloseProposalTransactionInstruction {}.data(),
    }
}

pub fn close_config_proposal(group: Pubkey, proposal: Pubkey, proposer: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![readonly(group), writable(proposal), writable(proposer)],
        data: ix_data::CloseProposalInstruction {}.data(),
    }
}

pub fn close_normal_proposal(group: Pubkey, proposal: Pubkey, proposer: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![readonly(group), writable(proposal), writable(proposer)],
        data: ix_data::CloseNormalProposalInstruction {}.data(),
    }
}

pub fn close_normal_vote_record(
    group: Pubkey,
    proposal: Pubkey,
    voter: Pubkey,
    asset_index: u8,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            readonly(group),
            readonly(proposal),
            writable(normal_vote_record_pda(&group, &proposal, &voter, asset_index).address),
            signer(voter),
        ],
        data: ix_data::CloseNormalVoteRecordInstruction {
            args: CloseNormalVoteRecordInstructionArgs { asset_index },
        }
        .data(),
    }
}

pub fn close_config_vote_record(group: Pubkey, proposal: Pubkey, voter: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            readonly(group),
            readonly(proposal),
            writable(config_vote_record_pda(&group, &proposal, &voter).address),
            signer(voter),
        ],
        data: ix_data::CloseConfigVoteRecordInstruction {}.data(),
    }
}

pub fn clean_up_asset_member(
    group: Pubkey,
    asset_address: Pubkey,
    member: Pubkey,
    rent_collector: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            readonly(group),
            writable(asset_member_pda(&group, &asset_address, &member).address),
            writable(asset_pda(&group, &asset_address).address),
            readonly(group_member_pda(&group, &member).address),
            readonly(member),
            writable(rent_collector),
        ],
        data: ix_data::CleanUpAssetMemberInstruction {}.data(),
    }
}

pub fn emergency_reset_proposal_pda(group: &Pubkey, proposal_seed: &Pubkey) -> Pda {
    pda(&[b"emergency-reset", group.as_ref(), proposal_seed.as_ref()])
}

pub fn emergency_reset_vote_record_pda(group: &Pubkey, proposal: &Pubkey, voter: &Pubkey) -> Pda {
    config_vote_record_pda(group, proposal, voter)
}

pub fn create_emergency_reset_proposal(
    args: CreateEmergencyResetProposalArgs,
    group: Pubkey,
    proposer: Pubkey,
) -> Instruction {
    let proposal = emergency_reset_proposal_pda(&group, &args.proposal_seed).address;
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            signer(proposer),
            writable(group),
            readonly(group_member_pda(&group, &proposer).address),
            writable(proposal),
            readonly(system_program::ID),
        ],
        data: ix_data::CreateEmergencyResetProposal { args }.data(),
    }
}

pub fn vote_on_emergency_reset_proposal(
    args: VoteOnEmergencyResetArgs,
    group: Pubkey,
    proposal: Pubkey,
    voter: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(proposal),
            readonly(group_member_pda(&group, &voter).address),
            writable(emergency_reset_vote_record_pda(&group, &proposal, &voter).address),
            signer(voter),
            readonly(system_program::ID),
        ],
        data: ix_data::VoteOnEmergencyResetProposal { args }.data(),
    }
}

pub fn execute_emergency_reset(group: Pubkey, proposal: Pubkey, proposer: Pubkey) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![writable(group), writable(proposal), writable(proposer)],
        data: ix_data::ExecuteEmergencyReset {}.data(),
    }
}

pub fn close_emergency_reset_proposal(
    group: Pubkey,
    proposal: Pubkey,
    proposer: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![readonly(group), writable(proposal), writable(proposer)],
        data: ix_data::CloseEmergencyResetProposal {}.data(),
    }
}

pub fn close_emergency_reset_vote_record(
    group: Pubkey,
    proposal: Pubkey,
    voter: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            readonly(group),
            readonly(proposal),
            writable(emergency_reset_vote_record_pda(&group, &proposal, &voter).address),
            signer(voter),
        ],
        data: ix_data::CloseEmergencyResetVoteRecord {}.data(),
    }
}

pub fn add_member_in_reset_mode(
    args: AddMemberInResetModeArgs,
    group: Pubkey,
    trusted_1: Pubkey,
    trusted_2: Pubkey,
    trusted_3: Pubkey,
    payer: Pubkey,
) -> Instruction {
    let new_member_account = group_member_pda(&group, &args.new_member).address;
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            writable(new_member_account),
            readonly_signer(trusted_1),
            readonly_signer(trusted_2),
            readonly_signer(trusted_3),
            signer(payer),
            readonly(system_program::ID),
        ],
        data: ix_data::AddMemberInResetMode { args }.data(),
    }
}

pub fn remove_member_in_reset_mode(
    group: Pubkey,
    member: Pubkey,
    trusted_1: Pubkey,
    trusted_2: Pubkey,
    trusted_3: Pubkey,
    rent_collector: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            readonly(member),
            writable(group_member_pda(&group, &member).address),
            readonly_signer(trusted_1),
            readonly_signer(trusted_2),
            readonly_signer(trusted_3),
            writable(rent_collector),
        ],
        data: ix_data::RemoveMemberInResetMode {}.data(),
    }
}

pub fn exit_pause_mode(
    args: ExitPauseModeArgs,
    group: Pubkey,
    trusted_1: Pubkey,
    trusted_2: Pubkey,
    trusted_3: Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            writable(group),
            readonly_signer(trusted_1),
            readonly_signer(trusted_2),
            readonly_signer(trusted_3),
        ],
        data: ix_data::ExitPauseMode { args }.data(),
    }
}

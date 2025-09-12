use anchor_lang::prelude::*;

#[error_code]
pub enum MultisigError {
    #[msg("Initial members must be exactly 5.")]
    InvalidInitialMembersLength,
    #[msg("Initial names must be exactly 5.")]
    InvalidNamesLength,
    #[msg("Initial URIs must be exactly 5.")]
    InvalidUrisLength,
    #[msg("The name provided is greater than 32.")]
    NameTooLong,
    #[msg("The URI provided is greater than 64.")]
    UriTooLong,
    #[msg("The config supplied does not match the instruction")]
    InvalidConfigChange,
    #[msg("The Member provided does not match the expected member")]
    InvalidMember, /// Change to unexpected if necessary
    #[msg("The proposal has not yet passed")]
    ProposalNotPassed,
    #[msg("The asset provided does not match the expected asset")]
    InvalidAsset, /// Change to unexpected if necessary
    #[msg("The provided member goes not govern the group")]
    NotGroupMember,
    #[msg("The provided member goes not govern the asset")]
    NotAssetMember,
    #[msg("An expected governed asset was not provided")]
    AssetNotProvided,
    #[msg("Configuration change does not match the expected")]
    UnexpectedConfigChange,
    #[msg("Too many assets for one proposal")]
    TooManyAssets,
    #[msg("Asset index out of range")]
    InvalidAssetIndex,
    #[msg("Proposal is not open")]
    ProposalNotOpen,
    #[msg("This voter does not have proper authorization")]
    UnauthorizedVoter,
    #[msg("This member collection is not tied to the the right asset/owner")]
    InvalidAssetMember,
    #[msg("Vote option not valid in this context")]
    InvalidVoteOption,
    #[msg("The group does not match what was expected")]
    UnexpectedGroup,
    #[msg("The asset does not match what was expected")]
    UnexpectedAsset,
    #[msg("An asset membership asset was not provided")]
    AssetMembershipNotProvided,
    #[msg("An group membership asset was not provided")]
    GroupMembershipNotProvided,
    #[msg("A group was not provided")]
    GroupNotProvided,
    #[msg("Member collection not provided")]
    AssetMemberNotProvided,
    #[msg("The voting window for the proposal has closed")]
    ProposalExpired,
    #[msg("The asset keys provided are not sorted or a duplicate is present")]
    AssetsNotSortedOrDuplicate,
    #[msg("Invalid initial weights provided.")]
    InvalidInitialWeights,
    #[msg("Invalid initial permissions provided.")]
    InvalidInitialPermissions,
    #[msg("Failed to deserialize instruction data")]
    InstructionDeserializationFailed,
    #[msg("Instruction hash is invalid")]
    InvalidInstructionHash,
    #[msg("Not enough account keys provided.")]
    NotEnoughAccountKeys,
    #[msg("The proposal is stale because a configuration has changed since it was created.")]
    ProposalStale,
    #[msg("The proposal is not stale so this instruction cannot be called.")]
    ProposalNotStale,
    #[msg("The proposal is not in a state that would allow for it to be closed.")]
    ProposalNotClosable,
    #[msg("Transaction not yet reached valid period")]
    TransactionNotRipe,
    #[msg("The Vectors/Arrays provided are of different lengths")]
    LengthMismatch,
    #[msg("Invalid threshold configuration")]
    InvalidThreshold,
    #[msg("Authority not Provided")]
    AuthorityNotProvided,
    #[msg("Authority not Set")]
    AuthorityNotSet,
    #[msg("Invalid mint mint authority")]
    InvalidMintMintAuthority,
    #[msg("Invalid mint freeze authority")]
    InvalidMintFreezeAuthority,
    #[msg("Invalid token owner")]
    InvalidTokenOwner,
    #[msg("Invalid token delegate")]
    InvalidTokenDelegate,
    #[msg("Invalid close authority")]
    InvalidCloseAuthority,
    #[msg("Invalid account state")]
    InvalidAccountState,
    #[msg("Invalid member count")]
    InvalidMemberCount,
    #[msg("State already finalized")]
    StateAlreadyFinalized,
    #[msg("Invalid state transition")]
    InvalidStateTransition,
    #[msg("Invalid proposer")]
    InvalidProposer,
    #[msg("Insufficient permissions")]
    InsufficientPermissions,
    #[msg("Invalid permissions")]
    InvalidPermissions,
    #[msg("Too many votes")]
    TooManyVotes,
    #[msg("Too many members")]
    TooManyMembers,
    #[msg("Group member is still active")]
    GroupMemberStillActive,
    #[msg("Proposal is still active")]
    ProposalStillActive,
    #[msg("Unexpected proposal")]
    UnexpectedProposal,
    #[msg("Unexpected rent collector")]
    UnexpectedRentCollector
}

/// Implement Into<ProgramError> for MultisigError
impl From<MultisigError> for ProgramError {
    fn from(e: MultisigError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

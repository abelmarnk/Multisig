# Solana Multisig Program

> This program is unaudited. Use of this code carries risk; do not run it with live funds or rely on it for security-critical operations. No warranties are provided.

A **Solana program** that implements a flexible, asset-aware multisig governance system.

---

## Table of Contents

1. [Overview](#overview)
2. [How It Differs from Standard Multisig](#how-it-differs-from-standard-multisig)
3. [Features](#features)
4. [Proposal Lifecycle in Detail](#proposal-lifecycle-in-detail)
5. [Multi-Instruction Proposals](#multi-instruction-proposals)
6. [Emergency Reset and Pause Mode](#emergency-reset-and-pause-mode)
7. [Design Decisions and Security Properties](#design-decisions-and-security-properties)
8. [SDK](#sdk)
9. [Workspace Layout](#workspace-layout)
10. [Prerequisites](#prerequisites)
11. [Building](#building)
12. [Tests](#tests)

---

## Overview

At its core, this program manages:

- **Groups** - organizational units that define membership, configuration rules, and proposal lifecycles.
- **Assets** - individual mints or token accounts, each with their own governance rules.
- **Members** - participants in a group who may also govern specific assets with varying weights and permissions.
- **Proposals** - suggested actions that members vote on.
- **Transactions** - executable instructions attached to approved normal proposals.

This model allows organizations to govern **multiple assets under one group**, while delegating control of individual assets to specific subsets of members.

---

## How It Differs from Standard Multisig

Traditional multisig systems enforce approval from one fixed set of signers for *every* action. This program enables **asset-specific governance**, where:

- **Group-level governance** - Adding/removing members, changing thresholds, timelocks, and expiry are governed by config proposals at the group level.
- **Asset-level governance** - Each asset inherits its group's context but can override configuration and assign governance to specific members.
- **Contextual authority** - Members of the same group can control different assets independently.
  - Example: An organization with 10 assets can delegate asset A to team X and asset B to team Y while both teams remain within the same group.

This design is especially suited for DAOs or organizations managing **many assets in different contexts**, where not every decision requires every member's approval.

---

## Features

The program provides a complete lifecycle of governance, voting, execution, and cleanup:

### 1. Group and Asset Management
- Create governance groups with configurable thresholds, timelocks, and expiry windows.
- Add or remove group members with weighted voting power and permissions.
- Register assets (mints or token accounts) under a group.
- Assign governance rights for specific assets to selected members.

### 2. Proposal Lifecycle
- Create **normal proposals** for executing transactions that use group-controlled assets.
- Create **config proposals** to update group or asset governance rules.
- Track proposals through states: `Open -> Passed / Failed -> Executed / Expired`.

### 3. Voting
- Members vote **for or against** proposals; weight is determined by their group or asset membership.
- Normal proposals track voting per asset, allowing independent thresholds per asset.
- Config proposals apply voting rules at the group or asset level.

### 4. Execution
- Execute proposal transactions once all thresholds are met and the timelock has elapsed.
- Authority is derived from program-derived addresses (PDAs), ensuring secure execution without private keys.

### 5. Minimum Timelock
- Groups define a `minimum_timelock` floor (in seconds). All normal and config proposals must declare a `timelock_offset` greater than or equal to this floor.
- The floor is changed via a config proposal using `ConfigType::MinimumTimelock(u32)` - it is a group-level setting and cannot be applied to assets.

### 6. Emergency Reset
- Any group member with Propose permission can open an **emergency reset proposal** at any time, even while the group is paused.
- The proposer commits three **trusted members** (PDA keys with weights and permissions) at creation time. Multiple emergency reset proposals can be open simultaneously using different `proposal_seed` values.
- Voting is **unanimous**: the proposal passes only when every current member has voted *for* it; it fails only when every current member has voted *against* it. Emergency reset proposals are immune to staleness - they are never invalidated by config changes.
- On execution, the group enters **pause mode**: `group.paused = true`, and the three trusted members are recorded on the group.

### 7. Pause Mode
- While `group.paused == true`, every normal instruction rejects with `GroupPaused`.
- Only three specialized instructions are available in pause mode, each requiring **all three trusted members to co-sign**:
  - `add_member_in_reset_mode` - initialise a new `GroupMember` PDA.
  - `remove_member_in_reset_mode` - close an existing `GroupMember` PDA.
  - `exit_pause_mode` - clear `paused` and resets configuration then validates that the resulting group is left in a valid state.

### 8. Cleanup and Rent Handling
- Close stale proposal transactions when governance conditions change.
- Close expired or failed proposals.
- Close asset member accounts once their parent group membership is removed.
- Close vote records once proposals are finalized.
- Rent from closed accounts flows back to the originator (proposer, voter) or to the group **rent collector** when there is no natural recipient.

---

## Proposal Lifecycle in Detail

### Normal Proposals

Normal proposals execute one or more on-chain instructions through PDA-controlled authorities. The lifecycle is strictly ordered to prevent footguns:

```
create_normal_proposal          (proposer commits instruction hashes)
        │
create_proposal_transaction     (anyone attaches the full instruction preimage)
        │
vote_on_normal_proposal         (members cast votes; gated on tx account existence)
        │
execute_proposal_transaction    (permissionless once passed + timelock elapsed)
        │
close_normal_proposal           (proposer reclaims rent)
```

> **Why are votes gated on `create_proposal_transaction`?**
>
> When a normal proposal is created, instruction hashes are committed on-chain - but the full instruction bytes (the preimage) are only verified and stored when `create_proposal_transaction` runs. Until that step completes, members would be voting on something whose consequences they cannot fully inspect. To eliminate this footgun, `vote_on_normal_proposal` requires the proposal-transaction PDA to exist and be owned by this program before a vote is accepted.
>
> If a proposer makes an error and decides not to attach instructions, the proposal can simply be ignored or closed once it goes stale or expires. No votes will have been cast on the incomplete proposal.

A normal proposal can be **closed** (by the proposer, to reclaim rent) if it is:
- Stale (the group configuration advanced past it).
- Expired (the deadline passed).
- In a terminal state (`Failed`, `Expired`, or `Executed`).

Note that the staleness/expiry checks also apply to the proposal-transaction account closure - these paths are intentionally left open so no funds are ever permanently locked.

### Config Proposals

Config proposals modify group or asset configuration (add/remove members, change thresholds, etc.). They do not involve an instruction preimage, so voting can begin immediately after creation.

---

## Multi-Instruction Proposals

Normal proposals support **multiple instructions per transaction**, enabling atomic sequences - for example, transferring tokens from multiple vaults in a single proposal.

### Commit/Reveal Design

The protocol uses a two-step commit/reveal scheme:

| Step | Instruction | What happens |
|------|-------------|--------------|
| 1 | `create_normal_proposal` | Proposer supplies `instruction_hashes: Vec<[u8; 32]>`, one hash per instruction. Hashes are stored on-chain and immediately visible to voters. |
| 2 | `create_proposal_transaction` | Anyone submits `raw_instructions: Vec<Vec<u8>>` (the full preimage). The program verifies each hash, validates asset PDAs and authority bumps, and stores the instructions. |

This ensures:
- Voters know **exactly** what will run before they vote (hashes are visible at proposal creation).
- The executor cannot substitute different instructions at execution time.
- Full validation of governed assets and authority bumps happens at attachment time, with complete instruction data available.

### Address Lookup Tables

Address lookup tables are **not supported** for proposal transactions. Every account required by each stored instruction must be supplied directly to `execute_proposal_transaction` as a remaining account.

### SDK Helpers

| Helper | Purpose |
|--------|---------|
| `serializable_instruction_hash(ix)` | Hash one `SerializableInstruction` |
| `serializable_instruction_hashes(ixs)` | Hash a slice -> use as `instruction_hashes` |
| `serializable_instruction_bytes(ix)` | Serialize one instruction -> use in `raw_instructions` |
| `serializable_instructions_bytes(ixs)` | Serialize a slice in one call |

### Error Reference

| Error | Cause |
|-------|-------|
| `EmptyInstructions` | `raw_instructions` is empty |
| `LengthMismatch` | Count of `raw_instructions` differs from stored `instruction_hashes` |
| `InvalidInstructionHash` | A raw instruction does not match its stored hash |
| `InvalidAsset` | A supplied governed asset PDA or authority bump does not match the proposal |
| `UnexpectedAsset` | The declared instruction/account index does not contain the expected asset key |

### Example Flow

```rust
// 1. Compute hashes at proposal creation time
let hashes = sdk::serializable_instruction_hashes(&[ix_a.clone(), ix_b.clone()])?;
let create_proposal_args = CreateNormalProposalInstructionArgs {
    instruction_hashes: hashes,
    // ...
};

// 2. In the same or a subsequent transaction, attach the instructions
let raw = sdk::serializable_instructions_bytes(&[ix_a, ix_b])?;
let attach_args = CreateProposalTransactionInstructionArgs { raw_instructions: raw };
let attach_ix = sdk::create_proposal_transaction(
    attach_args,
    group,
    proposal_seed,
    payer,
    &[asset_a, asset_b],
);
// After this succeeds, members may vote.
```

---

## Emergency Reset and Pause Mode

The emergency reset mechanism is designed for one scenario: a group's signing keys are compromised or become inaccessible and the membership must be rebuilt from scratch.

### Creating a Reset Proposal

A member with Propose permission calls `create_emergency_reset_proposal`, committing three trusted keys. There is no limit on how many reset proposals can be open at once.

### Voting

All active group members vote using `vote_on_emergency_reset_proposal`. The semantics are strict:
- **Pass** - every current member cast a *for* vote (`for_count == member_count`).
- **Fail** - every current member cast an *against* vote (`against_count == member_count`).

A proposal that has neither condition stays in `Open` state indefinitely or until it expires. Unlike normal and config proposals, emergency reset proposals **are not invalidated** by config changes advancing `proposal_index_after_stale` - they survive governance churn by design.

### Execution and Pause Mode

Once a proposal has Passed, anyone can call `execute_emergency_reset`:
1. `group.paused` is set to `true`.
2. The three trusted members defined at proposal creation are written into `group.reset_trusted_1/2/3`.
3. `group.proposal_index_after_stale` is advanced to `group.next_proposal_index`, marking all currently open normal/config proposals stale.
4. The proposal account is closed and rent returned to the proposer.

From this point on, every instruction that touches normal governance is rejected with `GroupPaused`. The only way to proceed is via the three trusted-member instructions.

### Rebuilding Membership

All three stored trusted keys must co-sign every call:

```
execute_emergency_reset
        │
        ▼  (group.paused = true)
add_member_in_reset_mode    <- repeat as needed
remove_member_in_reset_mode <- repeat as needed
        │
        ▼
exit_pause_mode             <- validates group state
                            <- clears group.paused and trusted fields
```

### Security Properties

| Guarantee | Mechanism |
|-----------|----------|
| Attacker cannot prevent reset | They would need to control **all** member votes to make every member vote against - any honest *for* vote blocks a unanimous-against fail |
| Attacker cannot force a reset | They would need to control **all** member votes (unanimous-for is required to pass) |
| Reset cannot be hijacked mid-flight | Trusted members are committed on-chain at proposal creation; cannot be changed after |
| Rebuild cannot be gamed | `exit_pause_mode` enforces that the group is in a valid state before unpausing |
| Multiple concurrent proposals | Different `proposal_seed` values yield independent proposals; whichever executes first wins |

---

## Prerequisites

| Tool | Version |
|------|---------|
| Rust | stable (edition 2021) |
| Solana CLI | compatible with Anchor 0.31.1 (≥ 1.18 recommended) |
| Anchor CLI | 0.31.1 |

Install the Anchor CLI:
```bash
cargo install --git https://github.com/coral-xyz/anchor anchor-cli --tag v0.31.1 --locked
```

---

## Building

### Full SBF program build (required before running integration tests)

```bash
anchor build
```

### Build the test-helper program (required for multi-instruction integration tests)

```bash
cargo build-sbf --manifest-path programs/multisig_test_helper/Cargo.toml
```

The test-helper is a minimal Solana program used by `integration_normal.rs` to test multi-asset proposals.

### Workspace type-check (fast, no SBF compilation)

```bash
cargo check
```

---

## Tests

Build both programs before running any test that loads `.so` files:
> ```bash
> anchor build
> cargo build-sbf --manifest-path programs/multisig_test_helper/Cargo.toml
> ```

### Run the full test suite

```bash
cargo test -p multisig --features test-helpers 2>&1
```

### Run a single test file

```bash
cargo test -p multisig --test vote_on_normal_proposal --features test-helpers
```

Replace `vote_on_normal_proposal` with any filename in `programs/multisig/tests/` (without the `.rs` extension).

### Run state-logic unit tests (no SBF build needed)

```bash
cargo test -p multisig --test state_logic
```

### Run the SDK tests

```bash
cargo test -p multisig-sdk
```

### Run everything in one command

```bash
anchor build && \
  cargo build-sbf --manifest-path programs/multisig_test_helper/Cargo.toml && \
  cargo test -p multisig --features test-helpers && \
  cargo test -p multisig-sdk
```

---

## Safety

This code is **unaudited** and must not be used to manage real funds or production assets.

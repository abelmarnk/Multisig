# Solana Multisig Program

This project is a **Solana anchor program** that implements a flexible, asset-aware multisig governance system, it is still in the process of being built
It's inspired by other projects, including:  

- [Squads](https://github.com/Squads-Protocol/v4)  
- [Vault pro](https://github.com/solana-turbin3/Q1_25_Builder_dvrvsimi)  

---

## Overview

At its core, this program manages:

- **Groups** – organizational units that define membership, configuration rules, and proposal lifecycles.  
- **Assets** – individual mints or token accounts that can each have their own governance rules.  
- **Members** – participants in a group, who may also govern specific assets with varying weights and permissions.  
- **Proposals** – suggested actions (normal or configuration changes) that members can vote on.  
- **Transactions** – executable instructions tied to approved proposals.  

This model allows organizations to govern **multiple assets under one group**, while delegating control of individual assets to specific subsets of members.  

---

## How It Differs from Standard Multisig

Traditional multisig systems enforce approval from the same set of signers across all actions.  
This program enables **asset-specific governance**, where:

- **Group-level governance** – Rules for adding/removing members, thresholds, timelocks, and expiry are governed by proposals at the group level.  
- **Asset-level governance** – Each asset inherits its group’s context but can override configuration and assign governance to specific members.  
- **Contextual authority** – Members of the same group can control different assets independently.  
  - Example: An organization with 10 assets can delegate asset A to team X and asset B to team Y, while both teams remain within the same group.

This design is especially suited for DAOs or organizations managing **many assets in different contexts**, where not every decision requires the approval of every member.

---

## Features

The program provides a complete lifecycle of **governance, voting, execution, and cleanup**:

### 1. Group and Asset Management
- Create governance groups with configurable thresholds, timelocks, and expiries.  
- Add or remove group members with weighted voting power and permissions.  
- Register assets (mints or token accounts) under a group.  
- Assign governance rights for specific assets to selected members.  

### 2. Proposal Lifecycle
- Create **normal proposals** for executing transactions that use assets.  
- Create **config proposals** to update group or asset governance rules.  
- Track proposals through states: **Open → Passed/Failed → Executed/Closed/Expired**.  

### 3. Voting
- Members vote **for or against** proposals, with weight determined by their group or asset membership.  
- Normal proposals track voting per asset, allowing granular thresholds for each asset.  
- Config proposals apply voting rules at the group or asset level.  

### 4. Execution
- Execute proposal transactions once thresholds are met.  
- Support for timelocks to delay execution until after a set time.  
- Execution may include:
  - **Transactions** involving program-controlled assets.  
  - **Configuration changes** (group-level or asset-level).  
  - **Membership changes** (adding/removing group or asset members).  
- Authority is derived from program-derived addresses (PDAs), ensuring secure execution.  

### 5. Cleanup and Rent Handling
- Close stale proposal transactions if governance conditions change before execution.  
- Close expired or failed proposals.  
- Close asset member accounts once their parent group membership is removed.  
- Close vote records once proposals are finalized.  
- Rent from closed accounts is distributed as follows:
  - If there is a clear recipient (e.g. the proposer for a proposal, or the voter for a vote record), the rent is refunded to them.  
  - If there is no appropriate recipient, the rent is sent to the group **rent collector** account.

---

## Building and Deploying
 - To be filled

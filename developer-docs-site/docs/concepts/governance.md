---
title: "Governance"
slug: "governance"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Governance

The Aptos on-chain governance is a process by which the Aptos community members can create and vote on proposals that minimize the cost of blockchain upgrades. The following describes the scope of these proposals for the Aptos on-chain governance:

- Changes to the blockchain parameters, for example, the epoch duration, and the minimum required and maximum allowed validator stake.
- Changes to the core blockchain code. 
- Upgrades to the Aptos Framework modules for fixing bugs or for adding or enhancing the Aptos blockchain functionality.
- Deploying new framework modules (at the address 0x1).

## How a proposal becomes ready to be resolved

See below for a summary description of how a proposal comes to exist and when it becomes ready to be resolved:

<ThemedImage
alt="Proposal voting flow"
sources={{
    light: useBaseUrl('/img/docs/voting-resolution-flow.svg'),
    dark: useBaseUrl('/img/docs/voting-resolution-flow-dark.svg'),
  }}
/>

- The  Aptos community can suggest an Aptos Improvement Proposal (AIP) in community forums,  channels and discuss them off-chain.
- When an off-chain AIP acquires sufficient importance, then an on-chain proposal can be created for the AIP via the `AptosGovernance` module. 
- Voters can then vote on this proposal on-chain via the `AptosGovernance` module. When the voting period is over, the proposal can be resolved.
- The proposal contains an early expiration threshold that is set to 50% of the total supply of Aptos Coins. This allows for emergency bug fixes **without waiting for the full voting period**, assuming that the votes of the 50% of the total supply are cast quickly. 
  - If the number of YES votes exceed this threshold, the proposal is ready to be resolved.
  - If the number of NO votes exceed this threshold, the proposal is considered failed. 

## Who can propose

- To either propose or vote, you must stake but you are not required to run a validator node. However, we recommend that you be a validator with a stake and be a part of the validator set. 
- To create a proposal, the proposer's backing stake pool must have the minimum required proposer stake. The proposer's stake must be locked up for at least as long as the proposal's voting period. This is to avoid potential spammy proposals. 
- Proposers can create a proposal by calling [`AptosGovernance::create_proposal`](https://github.com/aptos-labs/aptos-core/blob/27a255ebc662817944435349afc4ec33ea317e64/aptos-move/framework/aptos-framework/sources/aptos_governance.move#L183).

## Who can vote

- To vote, you must stake, though you are not required to run a validator node. Your voting power is derived from the backing stake pool. 
  
  :::tip
  
  Each stake pool can only be used to vote on each proposal exactly once.
  :::

- Voting power is calculated based on the current epoch's active stake of the proposer or voter's backing stake pool. In addition, the stake pool's lockup must be at least as long as the proposal's duration.




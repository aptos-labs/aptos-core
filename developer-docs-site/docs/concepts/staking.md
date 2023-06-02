---
title: "Staking"
slug: "staking"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Staking

:::tip Consensus
We strongly recommend that you read the consensus section of [Aptos Blockchain Deep Dive](./blockchain.md#consensus) before proceeding further. 
:::

In a distributed system like blockchain, executing a transaction is distinct from updating the state of the ledger and persisting the results in storage. An agreement, i.e., consensus, must be reached by a quorum of validators on the ordering of transactions and their execution results before these results are persisted in storage and the state of the ledger is updated. 

Anyone can participate in the Aptos consensus process, if they stake sufficient utility coin, i.e., place their utility coin into escrow. To encourage validators to participate in the consensus process, each validator's vote weight is proportional to the amount of validator's stake. In exchange, the validator is rewarded proportionally to the amount staked. Hence, the performance of the blockchain is aligned with the validator's interest, i.e., rewards.  

:::note 
Currently, slashing is not implemented.
:::

The current on-chain data can be found in [`staking_config::StakingConfig`](https://mainnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::staking_config::StakingConfig). The configuration set is defined in [`staking_config.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/configs/staking_config.move).

The rest of this document presents how staking works on the Aptos blockchain. See [Supporting documentation](#supporting-documentation) at the bottom for related resources.

## Staking on the Aptos blockchain

<!---
Below is a summary flow diagram of how staking on the Aptos blockchain works. The sections following the summary describe it in detail. 

<ThemedImage
  alt="Staking Flow"
  sources={{
    light: useBaseUrl('/img/docs/staking-light.svg'),
    dark: useBaseUrl('/img/docs/staking-dark.svg'),
  }}
/> --->

The Aptos staking module defines a capability that represents ownership. 

:::tip Ownership
See the `OwnerCapability` defined in [stake.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/stake.move).
:::

The `OwnerCapability` resource can be used to control the stake pool. Three personas are supported: 
- Owner
- Operator
- Voter

Using this owner-operator-voter model, a custodian can assume the owner persona and stake on the Aptos blockchain and participate in the Aptos governance. This model allows delegations and staking services to be built as it separates the account that is control of the funds from the other accounts (operator, voter), hence allows secure delegations of responsibilities. 

This section describes how this works, using Bob and Alice in the example. 

### Owner

The owner is the owner of the funds. For example, Bob creates an account on the Aptos blockchain. Now Bob has the `OwnerCapability` resource. Bob can assign his account’s operator address to the account of Alice, a trusted node operator, to appoint Alice as a validator.

As an owner:

- Bob owns the funds that will be used for staking.
- Only Bob can add, unlock or withdraw funds.
- Only Bob can extend the lockup period.
- Bob can change the node operator Alice to some other node operator anytime Bob wishes to do so.
- Bob can set the operator commission percentage.
- The reward will be deposited into Bob's (owner's) account.

### Operator

A node operator is assigned by the fund owner to run the validator node and receives commission as set by the owner. The two personas, the owner and the operator, can be two separate entities or the same. For example, Alice (operator) runs the validator node, operating at the behest of Bob, the fund owner.

As an operator:

- Alice has permissions only to join or leave the validator set.
- As a validator, Alice will perform the validating function.
- Alice has the permissions to change the consensus key and network addresses. The consensus key is used by Alice to participate in the validator consensus process, i.e., to vote and propose a block. Alice is allowed to change ("rotate") this key in case this key is compromised.
- However, Alice cannot move funds (unless Alice is the owner, i.e., Alice has the `OwnerCapability` resource).
- The operator commission is deducted from the staker (owner) rewards and deposited into the operator account.

### Voter

An owner can designate a voter. This enables the voter to participate in governance. The voter  will use the voter key to sign the governance votes in the transactions.

:::tip Governance
This document describes staking. See [Governance](./governance.md) for how to participate in the Aptos on-chain governance using the owner-voter model.
:::

## Validation on the Aptos blockchain

Throughout the duration of an epoch, the following flow of events occurs several times (thousands of times):

- A validator leader is selected by a deterministic formula based on the validator reputation determined by validator's performance (including whether the validator has voted in the past or not) and stake. **This leader selection is not done by voting.**
- The selected leader sends a proposal containing the collected quorum votes of the previous proposal and the leader's proposed order of transactions for the new block. 
- All the validators from the validator set will vote on the leader's proposal for the new block. Once consensus is reached, the block can be finalized. Hence, the actual list of votes to achieve consensus is a subset of all the validators in the validator set. This leader validator is rewarded. **Rewards are given only to the leader validator, not to the voter validators.**
- The above flow repeats with the selection of another validator leader and repeating the steps for the next new block. Rewards are given at the end of the epoch. 

## Validator state and stake state

States are defined for a validator and the stake. 

- **Validator state:** A validator can be in any one of these four states. Moreover, the validator can go from inactive (not tracked in the validator set anywhere) state to any one of the other three states: 
  - inactive
  - pending_active.
  - active.
  - pending_inactive.
- **Stake state:** A validator in pending_inactive or active state, can have their stake in either of these four states: 
  - inactive.
  - pending_active.
  - active.
  - pending_inactive. 
  
  These stake states are applicable for the existing validators in the validator set adding or removing their stake.

### Validator states

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/validator-state.svg'),
    dark: useBaseUrl('/img/docs/validator-state-dark.svg'),
  }}
/>

There are two edge cases to call out:
1. If a validator's stake drops below the required [minimum](#minimum-and-maximum-stake), that validator will be moved from active state directly to the inactive state during an epoch change. This happens only during an epoch change.
2. Aptos governance can also directly remove validators from the active set. **Note that governance proposals will always trigger an epoch change.**

### Stake state

The state of stake has more granularity than that of the validator; additional stake can be added and a portion of stake removed from an active validator.

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/stake-state.svg'),
    dark: useBaseUrl('/img/docs/stake-state-dark.svg'),
  }}
/>

### Validator ruleset

The below ruleset is applicable during the changes of state:

- Voting power can change (increase or decrease) only on epoch boundary.
- A validator’s consensus key and the validator and validator fullnode network addresses can change only on epoch boundary.
- Pending inactive stake cannot be moved into inactive (and thus withdrawable) until before lockup expires.
- No validators in the active validator set can have their stake below the minimum required stake.

## Validator flow

:::tip Staking pool operations
See [Staking pool operations](../nodes/validator-node/operator/staking-pool-operations.md) for the correct sequence of commands to run for the below flow.
:::

1. Owner initializes the stake pool with `aptos stake initialize-stake-owner`.
2. When the owner is ready to deposit the stake (or have funds assigned by a staking service in exchange for ownership capability), owner calls `aptos stake add-stake`.
3. When the validator node is ready, the operator can call `aptos node join-validator-set` to join the active validator set. Changes will be effective in the next epoch.
4. Validator validates (proposes blocks as a leader-validator) and gains rewards. The stake will automatically be locked up for a fixed duration (set by governance) and automatically renewed at expiration.
5. At any point, if the operator wants to update the consensus key or validator network addresses, they can call `aptos node update-consensus-key` or `aptos node update-validator-network-addresses`. Similar to changes to stake, the changes to consensus key or validator network addresses are only effective in the next epoch.
6. Validator can request to unlock their stake at any time. However, their stake will only become withdrawable when their current lockup expires. This can be at most as long as the fixed lockup duration.
7. After exiting, the validator can either explicitly leave the validator set by calling `aptos node leave-validator-set` or if their stake drops below the min required, they would get removed at the end of the epoch.
8. Validator can always rejoin the validator set by going through steps 2-3 again.
9. An owner can always switch operators by calling `aptos stake set-operator`.
10. An owner can always switch designated voter by calling `aptos stake set-delegated-voter`.

## Joining the validator set

Participating as a validator node on the Aptos network works like this: 

1. Operator runs a validator node and configures the on-chain validator network addresses and rotates the consensus key. 
2. Owner deposits her Aptos coins funds as stake, or have funds assigned by a staking service. The stake must be at least the minimum amount required.
3. **The validator node cannot sync until the stake pool becomes active.**
4. Operator validates and gains rewards. 
5. The staked pool is automatically be locked up for a fixed duration (set by the Aptos governance) and will be automatically renewed at expiration. You cannot withdraw any of your staked amount until your lockup period expires. See [stake.move#L728](https://github.com/aptos-labs/aptos-core/blob/00a234cc233b01f1a7e1680f81b72214a7af91a9/aptos-move/framework/aptos-framework/sources/stake.move#L728).
6.  Operator must wait until the new epoch starts before their validator becomes active.

:::tip Joining the validator set
For step-by-step instructions on how to join the validator set, see: [Joining Validator Set](../nodes/validator-node/operator/staking-pool-operations.md#joining-validator-set).
:::

### Minimum and maximum stake

You must stake the required minimum amount to join the validator set. Moreover, you can only stake up to the maximum stake amount. The current required minimum for staking is 1M APT tokens and the maximum is 50M APT tokens.

If at any time after joining the validator set, your current staked amount exceeds the maximum allowed stake (for example as the rewards are added to your staked amount), then your voting power and the rewards will be calculated only using the maximum allowed stake amount, and not your current staked amount. 

The owner can withdraw part of the stake and leave their balance below the required minimum. In such case, their stake pool will be removed from the validator set when the next epoch starts.

### Automatic lockup duration

When you join the validator set, your stake will automatically be locked up for a fixed duration that is set by the Aptos governance. 

### Automatic lockup renewal

When your lockup period expires, it will be automatically renewed, so that you can continue to validate and receive the rewards. 

### Unlocking your stake

You can request to unlock your stake at any time. However, your stake will only become withdrawable when your current lockup expires. This can be at most as long as the fixed lockup duration. You will continue earning rewards on your stake until it becomes withdrawable. 

The principal amount is updated when any of the following actions occur:
1. Operator [requests commission unlock](../nodes/validator-node/operator/staking-pool-operations.md#requesting-commission)
2. Staker (owner) withdraws funds
3. Staker (owner) switches operators

When the staker unlocks stake, this also triggers a commission unlock. The full commission amount for any staking rewards earned is unlocked. This is not proportional to the unlock stake amount. Commission is distributed to the operator after the lockup ends when `request commission` is called a second time or when staker withdraws (distributes) the unlocked stake. 

### Resetting the lockup

When the lockup period expires, it is automatically renewed by the network. However, the owner can explicitly reset the lockup. 

:::tip Set by the governance

The lockup duration is decided by the Aptos governance, i.e., by the covenants that the Aptos community members vote on, and not by any special entity like the Aptos Labs. 
:::

## Epoch

An epoch in the Aptos blockchain is defined as a duration of time, in seconds, during which a number of blocks are voted on by the validators, the validator set is updated, and the rewards are distributed to the validators. 

:::tip Epoch on Mainnet
The Aptos mainnet epoch is set as 7200 seconds (two hours).
:::

### Triggers at the epoch start

:::tip
See the [Triggers at epoch boundary section of `stake.move`](https://github.com/aptos-labs/aptos-core/blob/256618470f2ad7d89757263fbdbae38ac7085317/aptos-move/framework/aptos-framework/sources/stake.move#L1036) for the full code.
:::

At the start of each epoch, the following key events are triggered:

- Update the validator set by adding the pending active validators to the active validators set and by removing the pending inactive validators from the active validators set.
- Move any pending active stake to active stake, and any pending inactive stake to inactive stake.
- The staking pool's voting power in this new epoch is updated to the total active stake.
- Automatically renew a validator's lockup for the validators who will still be in the validator set in the next epoch.
- The voting power of each validator in the validator set is updated to be the corresponding staking pool's voting power.
- Rewards are distributed to the validators that participated in the previous epoch.

## Rewards

Rewards for staking are calculated by using:

1. The `rewards_rate`, an annual percentage yield (APY), i.e., rewards accrue as a compound interest on your current staked amount.
2. Your staked amount.
3. Your proposer performance in the Aptos governance.

:::tip Rewards rate
The `rewards_rate` is set by the Aptos governance. Also see [Validation on the Aptos blockchain](#validation-on-the-aptos-blockchain).
:::

### Rewards formula

See below the formula used to calculate rewards to the validator:

```
Reward = staked_amount * rewards_rate per epoch * (Number of successful proposals by the validator / Total number of proposals made by the validator)
```

### Rewards paid every epoch

Rewards are paid every epoch. Any reward you (i.e., validator) earned at the end of current epoch is added to your staked amount. The reward at the end of the next epoch is calculated based on your increased staked amount (i.e., original staked amount plus the added reward), and so on.

### Rewards based on the proposer performance

The validator rewards calculation uses the validator's proposer performance. Once you are in the validator set, you can propose in every epoch. The more successfully you propose, i.e., your proposals pass, the more rewards you will receive. 

Note that rewards are given only to the **leader-validators**, i.e., validators who propose the new block, and not to the **voter-validators** who vote on the leader's proposal for the new block. See [Validation on the Aptos blockchain](#validation-on-the-aptos-blockchain).

:::tip Rewards are subject to lockup period
All the validator rewards are also subject to lockup period as they are added to the original staked amount. 
:::

## Leaving the validator set

:::tip
See the Aptos Stake module in the Move language at [stake.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/stake.move).
:::

- At any time you can call the following sequence of functions to leave the validator set:
    - Call `Stake::unlock` to unlock your stake amount, and 
    - Either call `Stake::withdraw` to withdraw your staked amount at the next epoch, or call `Stake::leave_validator_set`.

## Rejoining the validator set

When you leave a validator set, you can rejoin by depositing the minimum required stake amount.

## Supporting documentation

* [Current on-chain data](https://mainnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::staking_config::StakingConfig)
* [Staking Pool Operations](../nodes/validator-node/operator/staking-pool-operations.md)
* [Delegation Pool Operations](../nodes/validator-node/operator/delegation-pool-operations.md)
* [Configuration file `staking_config.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/configs/staking_config.move)
* [Contract file `staking_contract.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/staking_contract.move) covering requesting commissions
* [All staking-related `.move files](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/framework/aptos-framework/sources)

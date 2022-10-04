---
title: "Staking"
slug: "staking"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Staking

## Concept

:::tip
We strongly recommend that you read the consensus section of the [Life of a Transaction](/guides/basics-life-of-txn#consensus) before proceeding further. 
:::

In a distributed system like blockchain, executing a transaction is different from updating the state of the ledger and persisting the results in storage. An agreement, i.e., consensus, must be reached by a quorum of validators on the ordering of transactions and their execution results before the results are persisted in storage and the state of the ledger is updated. 

A validator can participate in the consensus process. However, the validator can acquire the voting power only when they stake, i.e., place their utility coin into escrow. To encourage validators to participate in the consensus process, each validator's vote weight is made proportionate to the amount of validator's stake. In exchange, the validator is rewarded in proportion to the amount of validator's stake. Hence, the performance of the network, i.e., consensus, is aligned with the validator's interest, i.e., rewards.   

However, when a validator stakes a very large amount of the utility coin into escrow, it gives the validator a vote weight large enough to control the consensus outcome. This gives the validator the power to threaten the security of the blockchain network, for example, by approving a fraudulent transaction. In the Aptos blockchain, there is a limit to the amount any validator can stake, to prevent any single validator from turning rogue. Furthermore, staking mitigates such security attacks because fraudulent validators would have to be willing to forego rewards and even the valuation of their assets in order to attack the network.

In this way, staking in the Aptos blockchain drives the consensus while securing the blockchain network. 

The rest of this document presents how staking works on the Aptos blockchain.

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


### How a custodian can stake on Aptos

The Aptos staking module defines a capability that represents ownership. See [https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L85](https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L85).

This `OwnerCapability` resource can be used to control the stake pool. Three personas: the owner, the operator and the voter, are supported. Using this owner-operator-voter model, a custodian can assume the owner persona and stake on the Aptos blockchain, and participate in the Aptos governance. This model allows delegations and staking services to be built as the owner can provide funds to the validator and the voter personas.

This section describes how this works, using Bob and Alice in the example. 

#### Owner

The owner is the owner of the funds. For example, Bob creates an account on the Aptos blockchain. Now Bob has the `OwnerCapability` resource. Bob can assign his account’s operator address to the account of Alice, a trusted node operator, to appoint Alice as a validator.

As an owner:

- Bob owns the funds that will be used for staking.
- Only Bob can add or unlock or withdraw funds.
- Only Bob can extend the lockup period.
- Bob can change the node operator Alice to some other node operator anytime Bob wishes to do so.
- The reward will be deposited into Bob's (owner's) account.

#### Operator

A node operator is assigned by the fund owner to run the validator node. The two personas, the owner and the operator, can be two separate entities or the same. For example, Alice (operator) runs the validator node, operating at the behest of Bob, the fund owner.

As an operator:

- Alice has permissions only to join or leave the validator set.
- As a validator, Alice will perform the validating function.
- Alice has the permissions to change the consensus key and network addresses. The consensus key is used by Alice to participate in the validator consensus process, i.e., to vote and propose a block. Alice is allowed to change ("rotate") this key in case this key is compromised.
- However, Alice cannot move funds (unless Alice is the owner, i.e., Alice has the `OwnerCapability` resource.

#### Voter

An owner can designate a voter. This enables the voter to participate in governance. The voter  will use the voter key to sign the governance votes in the transactions.

:::tip Governance
This document describes staking. See [Governance](governance.md) for how to participate in the Aptos on-chain governance using the owner-voter model.
:::

## Validation on the Aptos blockchain

The following is a high-level description of how validation works on the Aptos blockchain:

- Throughout the duration of an epoch, the following flow of events occurs several times (thousands of times):
  - A validator leader is selected by a deterministic formula based on the validator reputation determined by validator's performance (including whether the validator has voted in the past or not) and stake. **This leader selection is not done by voting.**
  - The selected leader sends a proposal containing the collected quorum votes of the previous proposal and the leader's proposed order of transactions for the new block. 
  - All the validators from the validator set will vote on the leader's proposal for the new block. Once a quorum consensus is reached the block can be finalized. Hence the actual list of votes to achieve the quorum consensus is a subset of all the validators in the validator set. This leader validator is rewarded. **Rewards are given only to the leader validators, and not to the voters.**
  - The above flow repeats with the selection of another validator leader and repeating the steps for the next new block. Rewards are given at the end of the epoch. 

## Joining the validator set

Participating as a validator node on the Aptos network works like this: 

1. Run a validator node and configure the on-chain settings appropriately.
2. Deposit your Aptos coins funds as stake or have funds assigned by a staking service. The stake must be at least the minimum amount required.
3. Validate and gain rewards. 
4. Your stake will automatically be locked up for a fixed duration (set by the Aptos governance) and will be automatically renewed at expiration. You cannot withdraw any of your staked amount until your lockup period expires. See [https://github.com/aptos-labs/aptos-core/blob/00a234cc233b01f1a7e1680f81b72214a7af91a9/aptos-move/framework/aptos-framework/sources/stake.move#L728](https://github.com/aptos-labs/aptos-core/blob/00a234cc233b01f1a7e1680f81b72214a7af91a9/aptos-move/framework/aptos-framework/sources/stake.move#L728).

:::tip Joining the validator set
For step-by-step instructions on how to join the validator set, see: [Joining Validator Set](/nodes/validator-node/operator/connect-to-aptos-network#joining-validator-set).
:::

### Minimum and maximum stake

You must stake the required minimum amount to join the validator set. Moreover, you can only stake up to the maximum stake amount. 

If at any time after joining the validator set, your current staked amount exceeds the maximum allowed stake (for example as the rewards are added to your staked amount), then your voting power and the rewards will be calculated only using the maximum allowed stake amount, and not your current staked amount. 

The owner can withdraw part of the stake and leave their balance below the required minimum. In such case, their stake pool will be removed from the validator set when the next epoch starts.

### Automatic lockup duration

When you join the validator set, your stake will automatically be locked up for a fixed duration that is set by the Aptos governance. 

### Automatic lockup renewal

When your lockup period expires, it will be automatically renewed, so that you can continue to validate and receive the rewards. 

### Unlocking your stake

You can request to unlock your stake at any time. However, your stake will only become withdrawable when your current lockup expires. This can be at most as long as the fixed lockup duration. 

### Resetting the lockup

When the lockup period expires, it is automatically renewed by the network. However, the owner can explicitly reset the lockup. 

:::tip Set by the governance

The lockup duration is decided by the Aptos governance, i.e., by the covenants that the Aptos community members vote on, and not by any special entity like the Aptos Labs. 
:::

## Epoch

An epoch in the Aptos blockchain is defined as a duration of time, in seconds, during which a number of blocks are voted on by the validators, the validator set is updated, and the rewards are distributed to the validators. 

:::tip
For the AIT-3 an epoch on the Aptos blockchain is defined as 7200 seconds (two hours).
:::

### Triggers at the epoch start

:::tip
See [https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L862](https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L862) for the full code.
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
2. Your staked amount, and 
3. Your proposer performance in the Aptos governance.

:::tip Set by the governance
The `rewards_rate` is set by the Aptos governance.
:::

Also see [Validation on the Aptos blockchain](#validation-on-the-aptos-blockchain).

### Rewards formula

See below the formula used to calculate rewards to the validator:

```
Reward = staked_amount * rewards_rate per epoch * (Number of successful proposals by the validator / Total number of proposals made by the validator)
```

### Rewards paid every epoch

Rewards are paid every epoch. Any reward you (i.e., validator) earned at the end of current epoch is added to your staked amount. The reward at the end of the next epoch is calculated based on your increased staked amount (i.e., original staked amount plus the added reward), and so on.

### Rewards based on the proposer performance

The validator rewards calculation uses the validator's proposer performance. Once you are in the validator set, you can propose in every epoch. The more successfully you propose, i.e., your proposals pass, the more rewards you will receive. 

:::tip
All the validator rewards are also subject to lockup period as they are added to the original staked amount. 
:::

## Leaving the validator set

:::tip
See the Aptos Stake module in Move language here: [https://github.com/aptos-labs/aptos-core/blob/00a234cc233b01f1a7e1680f81b72214a7af91a9/aptos-move/framework/aptos-framework/sources/stake.move](https://github.com/aptos-labs/aptos-core/blob/00a234cc233b01f1a7e1680f81b72214a7af91a9/aptos-move/framework/aptos-framework/sources/stake.move)
:::

- At any time you can call the following sequence of functions to leave the validator set:
    - Call `Stake::unlock` to unlock your stake amount, and 
    - Either call `Stake::withdraw` to withdraw your staked amount at the next epoch, or call `Stake::leave_validator_set`.

:::tip Leaving the validator set
For step-by-step instructions on how to leave the validator set, see: [Leaving Validator Set](https://aptos.dev/nodes/ait/connect-to-testnet#leaving-validator-set).
:::

## Rejoining the validator set

When you leave a validator set, you can rejoin by depositing the minimum required stake amount.


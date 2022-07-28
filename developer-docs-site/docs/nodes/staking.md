---
title: "Staking"
slug: "staking"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Staking

## Concept

Staking drives the consensus while securing the blockchain network. Staking is both a requirement imposed on a validator and an incentive that aligns the validator’s interests with the security of the blockchain network. Below is a brief conceptual look at staking in general. 

Nodes of a special type, called validator nodes, distributed across the blockchain network, vote for the blocks to be included in the blockchain. In this way, the validators determine the next state of the blockchain. This is how distributed consensus is achieved in the authority-less world of blockchains. 

However, when a validator node acquires a very large amount of the blockchain coins, it gives the validator the power to threaten the security of the blockchain network, for example, by approving a fraudulent transaction. 

Staking is a requirement imposed on a validator in a way that solves two distinct problems in one stroke: 

1. A validator is required to temporarily place into an “escrow” account (i.e., stake) large amounts of their coin to be able to participate in consensus. This stake is the validator’s expression of integrity and a promise not to threaten the security of the blockchain. 
2. In exchange for locking up such significant amounts of coin into the “escrow” account, the validator is rewarded in proportion to the staked coins. This stake-and-reward scheme ensures the integrity of the consensus and discourages this validator from turning into a rogue validator. 
3. For any other rogue validator, this increases the costs of attacking the network, because the rogue validator will have to acquire coins considerably exceeding the maximum staked coins. This cost is usually prohibitively high and hence this ensures the security of the blockchain network.

The rest of this document presents how staking works on the Aptos blockchain.

## Staking on the Aptos blockchain

Below is a summary flow diagram of how staking on the Aptos blockchain works. The sections following the summary describe it in detail. 

<ThemedImage
  alt="Staking Flow"
  sources={{
    light: useBaseUrl('/img/docs/staking-light.svg'),
    dark: useBaseUrl('/img/docs/staking-dark.svg'),
  }}
/>

## Joining the validator set

If you are running a validator node, then you can participate in consensus on the Aptos blockchain in a fully permissionless way by joining the validator set. However, to join a validator set:

1. You must stake your Aptos coins with at least the minimum amount required, and
2. You must lock up these staked coins for at least the minimum lockup duration required. You cannot withdraw any of your staked amount until your lockup period expires.

When you satisfy the above two minimum requirements, then you can join the validator set at any time, start validating and earn rewards.

:::tip Joining the validator set
For step-by-step instructions on how to join the validator set, see: [Joining Validator Set](https://aptos.dev/nodes/ait/connect-to-testnet#joining-validator-set).
:::

### Minimum and maximum stake

You must stake the required minimum amount to join the validator set. Moreover, you can only stake up to the maximum stake amount. 

If at any time after joining the validator set, your current staked amount exceeds the maximum allowed stake (for example as the rewards are added to your staked amount), then your voting power and the rewards will be calculated only using the maximum allowed stake amount, and not your current staked amount. 

### When the staked amount falls below minimum

If after joining the validator set, at the start of an epoch your stake drops below the minimum required amount, then you will be removed from the validator set. 

:::tip
In the current version of the staking on the Aptos blockchain, there is no possibility of your stake dropping below the required minimum before the lockup period expires. **This will change when Aptos implements slashing, i.e., penalty for malicious validator behavior**.
:::

### Minimum and maximum lockup period

To join the validator set, you must lockup your stake for the required minimum lockup period (in seconds). You cannot withdraw any staked amount before the expiry of the lockup period. 

Moreover, you can only lockup your stake for the maximum allowed period (in seconds).

However, if your remaining lockup time falls below the required minimum lockup period, then you will **not** be removed from the validator set. 

:::tip Note the difference
If your staked amount falls below the required minimum you will be removed from the validator set, but if your remaining lockup time falls below the required minimum lockup period, you will not be removed from the validator set.
:::

### When the lockup period expires

When your lockup period expires, you can either extend the lockup period so you can continue to validate and receive the rewards, or you can withdraw your total staked amount and stop validating.

:::tip Set by the governance

The above minimum and maximums for staked amount and the lockup period are decided by the Aptos governance and not by any special entity like the Aptos Labs. These minimum and maximum configurations are controlled by the covenants that the Aptos community members vote on.
:::

## Epoch

An epoch in the Aptos blockchain is defined as a duration of time, in seconds, during which a number of blocks are voted on by the validators, the validator set is updated, and the rewards are distributed to the validators. 

:::tip
Currently an epoch on the Aptos blockchain is defined as 3600 seconds (one hour).
:::

### Triggers at the epoch start

:::tip
See [https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L862](https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L862) for the full code.
:::

At the start of each epoch, the following key events are triggered:

- Update the validator set by adding the pending active validators to the active validators set and by removing the pending inactive validators from the active validators set.
- The status of the stake is updated. For example any pending active stake is updated to active stake and any pending inactive stake is updated to inactive stake.
- The voting power of the validator in the validator set is updated.
- Rewards are distributed to the active validators (i.e., validators in the validator set).
- Rewards are distributed to the validators who requested to leave but have not yet been removed.

## Rewards

Rewards for staking are calculated by using `rewards_rate`, an annual percentage yield (APY), using the below two numbers:

- Your current total staked amount.
- Your remaining lock up time.

Rewards accrue as a compound interest on your current staked amount. 

:::tip Set by the governance

The `rewards_rate` is set by the Aptos governance.

:::

### Rewards paid every epoch

Rewards are paid every epoch. Any reward you earned at the end of current epoch is added to your staked amount. The reward at the end of the next epoch is calculated based on your increased staked amount (i.e., original staked amount plus the added reward), and so on.

### Rewards formula

The lock up period has both the minimum required time and the maximum allowed time. If you lock up for the maximum period, then you will receive the maximum `rewards_rate`, i.e., a measure of APY. amount of the possible rewards. For example, if the APY is 10%, then if you lock up for the full maximum period, then you will receive the maximum reward, calculated at 10% APY on the staked amount in your account.

Shown below the formula used to calculate your rewards:

```
Reward = Maximum possible reward * (Remaining lockup / Maximum lockup) * (Number of successful votes / Total number of blocks in the current epoch)
```

The quantity `Maximum possible reward * (Remaining lockup / Maximum lockup)` is the `rewards_rate`, hence the `rewards_rate` will increase if you increase the remaining lockup period, eventually reaching the maximum when the remaining lockup period is the same as the maximum lockup period.

### Rewards use the remaining lockup period

As you can see above, the `rewards_rate` calculation formula is based on the remaining lockup period. For example, when you started with two years of lockup period, at the start your remaining lockup period is two years. After three days (`3*24` epochs) your remaining lockup period will be two years minus three days. 

If you do not extend your lock up period, then the remaining lockup period will decrease linearly over time, eventually becoming zero at the end of the two years. In this case, after the two years have elapsed your lockup period is zero and hence you will no longer receive any rewards.

### Rewards based on the voting performance

Your rewards calculation also uses your voting performance. Once you are in the validator set, you can vote in every epoch. The more consistently you vote, i.e., vote in every epoch, without any missed votes, you will receive additional voting power. This voting power is used to calculate your rewards. 

For every epoch, your voting performance is determined as follows:

- A running count of your missed votes, `validator_missed_votes_counts`, is maintained.
- The number of successful votes cast by you is calculated as:

```
Total number of successful votes = Total number of blocks in the epoch - Total number of your missed votes in the current epoch
```

Hence:

```
Reward = rewards_rate * (Number of successful votes / Total number of blocks in the current epoch)
```

:::tip
A validator’s missed votes count does not affect whether the validator is in the validator set or not. The missed votes count is used only to calculate the rewards, using the above formula.
:::

### Maintaining high rewards

You can prevent your rewards from gradually declining by regularly extending your lockup period. You can extend or renew your lockup period any time in a permissionless way.

For example, if you locked up for two years. A month from now you will receive a little less reward because your remaining lockup period will then be less (two years minus one month). However, if, before the month has fully elapsed, you extend your lockup period by one month to bring it back up to two years, then your month-end rewards will not decrease as they will be calculated based on the extended lockup period of two years.

:::tip
All your rewards are also subject to lockup period as they are added to the original staked amount. Hence you cannot withdraw your rewards until your lockup period has entirely expired.
:::

## Leaving the validator set

You can leave the validator set in the following ways:

:::tip
See the Aptos Stake module in Move language here: [https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/configs/stake.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/configs/stake.move)
:::

- When your lockup period expires, you can choose not to extend it and instead call the following sequence of functions to leave the validator set:
    - Call `Stake::unlock` to unlock your stake amount.
    - Call `Stake::withdraw` to withdraw your staked amount at the next epoch, and finally
    - Call `Stake::leave_validator_set` to be removed from the validator set.
- When your lockup period is not expired, you can leave the validator set. However, in this case you cannot unlock or withdraw your stake.
- Even when your lockup period has not expired, if, at the start of an epoch, your staked amount falls below the minimum required stake, then you will be automatically removed from the validator set at the start of the same epoch.
  - In the current version of the staking on the Aptos blockchain, there is no possibility of your stake dropping below the required minimum before the lockup period expires. **This will change when Aptos implements slashing, i.e., penalty for malicious validator behavior.** 
    

:::tip Leaving the validator set
For step-by-step instructions on how to leave the validator set, see: [Leaving Validator Set](https://aptos.dev/nodes/ait/connect-to-testnet#leaving-validator-set).
:::

## Rejoining the validator set

When you leave a validator set, you can rejoin by depositing the minimum required stake amount for the minimum required lockup period.

## How a custodian can stake on Aptos

The Aptos staking module defines a capability that represents ownership. See [https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L85](https://github.com/aptos-labs/aptos-core/blob/0daade4f734d1ba29a896b00d7ddde2249e87970/aptos-move/framework/aptos-framework/sources/configs/stake.move#L85).

This `OwnerCapability` resource can be used to control the node operator (i.e., who runs the validator node) and the associated stake pool.

Using this owner-operator model, a custodian can stake on the Aptos blockchain. This allows delegations and staking services to be built as the owner can provide funds to the validator.

This section describes how this works, using Bob and Alice in the example. 

### Owner

- Bob creates an account on the Aptos blockchain. Now Bob has the `OwnerCapability` resource.
- Bob can assign his account’s operator address to the account of Alice, a trusted node operator, to appoint Alice as a validator.

As an owner:

- Bob owns the funds that will be used for staking.
- Only Bob can add or remove funds.
- Only Bob can extend or renew the lockup period.
- Bob can change the node operator Alice to some other node operator anytime Bob wishes to do so.

### Operator

A node operator is assigned by the fund owner to run the validator node. These two entities, the owner and the operator, can be a single entity. 

In this example, Alice runs the validator node, operating at the behest of Bob, the fund owner.

As an operator:

- Alice has permissions only to join or leave the validator set.
- As a validator, Alice will perform the validating function.
- Alice has the permissions to change the consensus key and network addresses. The consensus key is used by Alice to participate in the validator consensus process, i.e., to vote and propose a block. Alice is allowed to change ("rotate") this key in case this key is compromised.
- However, Alice cannot move funds (unless Alice is the owner, i.e., Alice has the `OwnerCapability` resource.

---
title: "Staking on Aptos"
slug: "staking-on-aptos"
---

# Staking on Aptos

This document presents a conceptual introduction to staking. 

Staking is at the heart of the distributed consensus in the authority-less world of public blockchains. In particular, staking makes possible a particular type of distributed consensus mechanism called **proof of stake** consensus. 

## Transaction integrity

When you submit a transaction to a blockchain, the desired outcome is **all** of the following:

1. All the events defined in the transaction occur with integrity.
2. These events are permanently recorded in a secure and immutable way in the blockchain, and, 
3. The distributed copies of the blockchain are updated with the settled transaction details. 

However, there is no single trusted central authority in a distributed blockchain. Hence, determining who will guarantee, and how, these desired outcomes is a key **problem**.

This problem is solved by making use of a concept called distributed consensus. 

## Proof of stake

The proof of stake is a type of the distributed consensus mechanism, and it works like this: 

:::tip
The Aptos Blockchain uses the proof of stake distributed consensus mechanism.
:::

Some nodes, called Validators, are given temporary authority to decide if the next set of transactions in a block are correct. Only after a Validator validates these transactions will the block be included in the blockchain. 

The authority is **temporary** because the same Validator cannot continue to validate transaction after transaction in block after block. Allowing the same Validator to do so will start to look like the Validator is a central authority, effectively monopolizing and controlling the distributed consensus mechanism.

:::tip 

Distributed consensus mechanisms are usually deployed to work only within the scope of a blockchain.
:::

### Validators selected

- Validators willing and capable of participating in the distributed consensus will store some of their own coins in an intermediate storage of the blockchain. This storing of the coins is a proof, an expression of the Validator's intent to serve as a Validator, and is called **staking**. 
- The more coins a Validator stakes, the stronger is the Validator's intent to be selected—hence, the better the Validator's chances of being selected.
- The blockchain will select a Validator by using the criterion called **staking age**. The staking age of each Validator's staked amount. **Staking age = Staked coin amount x the duration (in days) these coins are held as stake.** Hence the staking age shows the dedication and commitment of the Validator to be selected.
- From the set of the selected Validators, a Validator is selected for each next block. 

### Validation

- The Validator will then verifies the next block, signs the transactions in the block, and adds the block to the blockchain. The amount of reward this Validator receives will be a `%` of transaction fee in the block, paid by the submitters of the transactions contained in the block. Hence, the Validator's reward may vary from block to block.
- If the Validator is involved in a fraudulent transaction, then the Validator will lose a portion of its stake. More importantly, the Validator will lose the right to be considered in the future. For example, when the staked amount is larger than the reward amount then the fraudulent  Validator may incur a net loss and will be blacklisted in the blockchain.

### After validation is done

- After a Validator validates a block, the Validator receives the reward. The Validator may choose to add this reward to the staked amount or not.
- The age of the Validator's staked coins is reset to zero. This is to ensure that the Validator's staking age does not increase continuously without bounds.
- This Validator must wait for some time before it can validate again. This ensures that just because this Validator's staking age is large, it cannot continue to validate block after block after block.
- The process will begin for validating the next round of transactions. 
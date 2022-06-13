---
title: "Aptos Mental Model"
id: "aptos-mental-model"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Aptos Blockchain: A Mental Model

## Overview

This section presents a conceptual view, a mental model, of the Aptos Blockchain and how you interact with it. This section is intended for a beginner developer on the Aptos Blockchain.

Read this before you get into the details of how to write an application for the Aptos Blockchain.

:::caution IMPORTANT 
This section is highly simplified, meant only to provide a conceptual mental model. Many important details are omitted to focus on a few key high-level ideas on how to think of the Aptos Blockchain.

:::

The Aptos Blockchain is a permissionless, or public, blockchain. This blockchain already exists in [the Aptos devnet](https://explorer.devnet.aptos.dev/), which means you can start interacting with the devnet immediately.

## Transactions

Every time you interact with the Aptos Blockchain, or any blockchain, at a high-level you should think of this interaction in the following terms: 

- Identify who you are, i.e., specify your account credentials.
- What kind of operation you want to perform in this interaction with the blockchain. For example, you can send money to someone or receive money from someone else. You can create an NFT (non-fungible token), or you can create and deploy a DApp (distributed application) using a smart contract. 
- Any such operation will be done by means of **transactions**. For example, sending money to someone will involve constructing a single transaction. Similarly, receiving money from someone will also require a single transaction, and so on. 
- Hence, as a developer, to interact with the Aptos Blockchain you will construct a set of transactions you need for the specific operation you wish to perform, and execute these transactions. When your transaction is successfully completed, the Aptos Blockchain is updated with the record of the transaction. 

:::tip
The Aptos developer tools and the developer documentation give you everything you need to get started with your development. You do not need to know anything about the Aptos Blockchain before you start.
:::

## Fees

You can create your account on the Aptos devnet for free, but to conduct any transaction with the Aptos Blockchain on the devnet, you are required to pay some processing fees. Think of this processing fee in the following terms: 

- Any blockchain, for example, the Aptos Blockchain, exists in a distributed network of computing and storage resources. The Aptos devnet is one such distributed network, holding the Aptos Blockchain. The Aptos devnet exists so that the developers can experiment on it, for example, to write and test DApps for the Aptos Blockchain, in preparation for the Aptos Mainnet launch.
- The fee you pay will be used to utilize these computing, networking and storage resources to:
  - Process your transactions on the blockchain.
  - Propagate the validated record throughout the distributed network, and
  - Store the validated record in the distributed blockchain storage.  
- Conceptually, this fee is modeled quite similar to how we pay for our home electricity or water utility. 
   - The blockchain provider publishes on their website the current going rate, i.e., price per unit of resource consumption. 
   - Every time your transaction is completed, the blockchain will count how many units of resources your transaction consumed.
   - At the successful completion of your transaction, a fee that is equivalent to `total units consumed by your transaction * price per unit` will be deducted from your account. 

### Gas unit

You can make a simple transaction, or a complicated transaction that requires the blockchain to perform lots of computation, network communication and distributed storage. In either case, you will be required to spend a processing fee sufficient to complete the transaction. 

You can even bump up your transaction to a higher priority level on the blockchain by paying a larger processing fee. This is where the notion of **gas** comes into play. Here is how it works:

- In the blockchain world, a **gas unit**, or a **unit of gas**, represents a basic unit of resource. A single unit of gas is a combined representation of:
    - A single unit of computation resource.
    - A single unit of network communication resource, and
    - A single unit of storage resource. 
- When your transaction is executed on the blockchain, instead of separately keeping track of each unit of computation, communication and storage, the blockchain simply keeps an account of the number of units of gas being consumed by the transaction. 
- The blockchain provider publishes the current going rate, i.e., price per unit of resource consumption, in terms of **price per unit of gas,** or **the gas price.** It is common to see fluctuations in the gas price, similar to any price in the real world that is subject to market fluctuations. See [Ethereum Gas Tracker](https://etherscan.io/gastracker), for example, which shows the real-time Ethereum gas price movements. 
- In your transaction, you can commit to paying a gas price that is higher than the market gas price. This is one way you can move your transaction higher in the priority list and have it  processed quicker. 

:::tip 👉 **Recap**
A **gas unit** is a dimensionless number, expressed in integers. The total gas units consumed by your transaction depends on the complexity of your transaction. The **gas price**, on the other hand, is published by the blockchain and expressed in terms of the blockchain’s native coin.
:::

:::caution **Advanced** 
See [Transactions and States](/docs/concepts/basics-txns-states.md) for how a transaction submitted to the Aptos Blockchain looks like.
:::

### Currency of the gas price

The gas price published by the blockchain is expressed in units of the blockchain’s native coin. Hence you will pay transaction processing fee in the currency of the blockchain’s native coin. See [Ethereum Gas Tracker](https://etherscan.io/gastracker), for example.

### Obtaining the currency

A blockchain will issue a native coin when the blockchain Mainnet is live, i.e., the Mainnet is actively accepting and validating transactions in the real world. 

Developers and users interacting with a Mainnet of a blockchain often purchase the blockchain’s native coins using a real currency, such as US dollar, and use these coins to pay for the gas price.

For the Aptos Blockchain, the Mainnet does not exist yet. Instead, the Aptos community of developers interact with the Aptos devnet. The currency on the devnet is the test Aptos coins, which have no monetary value and exist only for test purposes.

#### Devnet Faucet

On the Aptos devnet you can use the Faucet service to create **test** Aptos coins. These test Aptos coins are like play money. You can create them for free and use them to pay for the transaction fee on the Aptos devnet. The test Aptos coins have **no value in the real world** but they can be used in the experimental development projects on the Aptos devnet. 

:::tip 👉🏽 Faucet
The Faucet service on the Aptos devnet is similar to the Faucet service on Ethereum that [issues fake ETH](https://ethereum.org/en/developers/tutorials/hello-world-smart-contract/#step-4).
:::

## Accounts

Before you can do anything with the Aptos Blockchain, you must create an account on the Aptos Blockchain. 

## Signing a transaction

In every transaction you are involved in, you not only must provide your account address, but you must also digitally sign the transaction. 

Signing a transaction serves the same purpose of verifying the account holder’s identity. A simplistic analogy to signing a transaction on the Aptos Blockchain is entering an OTP code, or the code from a mobile authentication app, to authenticate the bank transaction.

## Account as a container

Conceptually similar to your bank account, think of your account on the Aptos Blockchain as a container of all the information your account is involved in. For example, when you use the Aptos Faucet service to create test Aptos coins, the play money we discussed earlier, the Faucet service deposits these coins into your account. 

As you begin to develop applications for the Aptos Blockchain using the Move language, you will see that this container concept of an account plays a prominent role. For example, an account is a container that holds Move resources and Move modules.  

:::caution **Advanced**
See [Accounts](https://aptos.dev/basics/basics-accounts) for how an account is represented on the Aptos Blockchain.

:::

## Validating a transaction

Say you created an account and issued a few test coins (play money) for yourself with the Faucet service. You then wrote up a transaction to send a few of your test coins to your partner who has an account on the Aptos blockchain. You made sure that you had issued enough test coins for yourself to pay for the transaction fee also. You signed the transaction and submitted it to devnet. What happens next? More important, how can you be sure that your transaction is successful?

This is where **Validator** nodes come in. 

- The Aptos devnet is actually a distributed network comprised of many nodes. Copies of the blockchain exists on each of these nodes. When a transaction is successful the state of the blockchain changes, with the new state of the blockchain containing the transaction information.
- Some nodes, called Validator nodes, perform the function of making sure that transactions are processed (validated). These Validator nodes use what is known as a consensus function to validate the transactions. The Validator node validates the transaction and proposes to update the blockchain with this validated transaction.
- Each node in the devnet will then update its copy of the blockchain with the latest version. So now we are again in a state where all the nodes in the devnet contain the exact copy of the blockchain, which is what we want.

A node in the Aptos devnet is a physical computer but it can be configured as either a **Validator** role or a **FullNode** role, or both. Without going into details, here is a simplistic summary of how these roles interact:

- When you submit your transaction, you are really submitting the transaction to a FullNode. The transaction will be added to the **Mempool** in the FullNode. A mempool is nothing but an in-memory buffer of all the transactions that have come into the blockchain from the users to this FullNode. Transactions in the Mempool have not been processed by the validator yet. 
    
:::info 👉🏽 FullNode as a "gateway"
All submitted transactions will enter the Aptos Blockchain through a FullNode. Hence, a FullNode acts as a gateway node through which the transactions enter the Aptos Blockchain.
:::
    
- This FullNode will then forward the transaction to a Validator node and will wait for the communication back from the Validator on whether the transaction has been processed successfully or not. 
- Consider the Validator node. After you submit a transaction to the devnet, all the critical action happens on a Validator node. For example, the Validator will:
    - Validate or reject the transaction.
    - Keep track of the gas units consumed by your transaction. If the transaction has already consumed the maximum gas amount you allotted in your transaction, and if your transaction is still not complete, then the Validator will discard the entire transaction. This is because all transactions on a blockchain must be atomic transactions, i.e., either execute the entire transaction or not at all. Partial transaction execution is not allowed. See **[atomic transactions](https://en.wikipedia.org/wiki/Atomicity_%28database_systems%29)**. 
- Hence, a complex set of compute operations and communication events are involved in a Validator node function. Compute resources such as processing power and storage, and communication and synchronization among the distributed nodes containing distributed databases (holding the blockchain) must be managed in a safe and secure manner.
- However, you, as an application developer on Aptos Blockchain, can submit only the details of your transaction (the "what") and not the details of **how** the Validator node should manage such distributed compute and communication operations needed to validate your transaction. This is where the **Move Virtual Machine** (Move VM) comes in. See [Move on Aptos](/guides/move-guides/move-on-aptos).

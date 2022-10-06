---
title: "Using Aptos Explorer"
slug: "use-aptos-explorer"
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Using Aptos Explorer

The [Aptos Explorer](https://explorer.aptoslabs.com/] lets you delve into the activity on the Aptos blockchain in great detail, seeing transactions, validators, and account information. With the Aptos Explorer, you may ensure the work you do in the [Aptos Petra Wallet](install-petra-wallet.md) and elsewhere on the blockchain is accurately reflected in Aptos.

The Aptos Explorer provides a one-step search engine across the blockchain to discover details about wallets, transactions, network analytics, user accounts, smart contracts, and more. The Aptos Explorer also offers dedicated pages for key elements of the blockchain and acts as the source of truth for all things Aptos.

## Users

The Aptos Explorer gives you a near real-time view into the status of the network and the activity related to the core on-chain entities. It serves these audiences and purposes by letting:

* App developers understand the behavior of the smart contracts and sender-receiver transaction flows.
* General users view and analyze Aptos blockchain activity on key entities - transactions, blocks, accounts, and resources.
* *ode operators check the health of the network and maximize the value of operating the node.
* Token holders find the best node operator to delegate the tokens and earn a staking reward.

## Common tasks

Follow the instructions here to conduct typical work in the Aptos Explorer.

### Select network

The Aptos Explorer renders data from all Aptos networks: Mainnet, Testnet, Devnet, and your local host if configured. See [Aptos Blockchain Deployments](../nodes/aptos-deployments.md) for a detailed view of their purposes and differences.

To select a network in the [Aptos Explorer](https://explorer.aptoslabs.com/], load the explorer and use the *Select Network* drop-down menu at the top right to select your desired network.

### Find a transaction

One of the most common tasks is to track a transaction in Aptos Explorer. You may search by the account address, transaction version and hash, or block height and version.

To find a transaction:

1. Enter the value in the *Search transactions* field near the top of any page.
1. Do not press return.
1. Click the transaction result that appears immediately below the search field, highlighted in green within the following screenshot:
<center>
<ThemedImage
alt="Search Aptos Explorer for transaction"
sources={{
    light: useBaseUrl('/img/docs/1-explorer-search-txn.png'),
    dark: useBaseUrl('/img/docs/1-explorer-search-txn-dark.png'),
  }}
/>
</center>

The resulting [Transaction details](#transaction-details) page appears.

## Explorer pages

This secton walks you through the available screens in Aptos Explorer to help you find the information you need.

### Explorer home

The Aptos Explorer home page provides an immediate view into the total supply of Aptos coins, those that are now staked, transactions per second (TPS), and active validators on the network, as well as a rolling list of the latest transactions:

<center>
<ThemedImage
alt="Aptos Explorer home page"
sources={{
    light: useBaseUrl('/img/docs/2-explorer-home.png'),
    dark: useBaseUrl('/img/docs/2-explorer-home-dark.png'),
  }}
/>
</center>

Click the **Transactions** tab at the top or  **View all Transactions** at the bottom to go to the [Transactions](#transactions) page.

### Transactions

The *Transactions* page displays all transactions on the Aptos blockchain in order, with the latest at the top of an ever-growing list.

In the transactions list, single click the **Hash** column to see and copy the hash for the transaction or double click the hash to go directly to the transaction details for the hash.

Otherwise, click anywhere else in the row of the desired transaction to load its [Transaction details](#transaction-details) page.

TODO: Find out the differences of and purposes for the regular transaction details page and the one specific to the hash:

https://explorer.aptoslabs.com/txn/159716025

https://explorer.aptoslabs.com/txn/0xf48137d98f00b9f574801c22ca33842795df8b4e7753260bd40c2b7677b93846

Use the controls at the bottom of the list to navigate back through transactions historically.

#### Transaction details

##### Overview

##### Events


##### Payload

##### Changes

### Accounts

### Blocks

### Validators
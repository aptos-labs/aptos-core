---
title: "Wallet Extension"
slug: "building-wallet-extension"
sidebar_position: 12
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Building the Wallet Extension

This tutorial goes through how to build the wallet extension and how to use it with your dApp 
1. Install the wallet on Chrome
2. Wallet functionality
3. dApp Integration

The code for the wallet can be found on our [github](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/web-wallet).

## Step 1) Install the wallet on Chrome

1. Download the latest [wallet release](https://github.com/aptos-labs/aptos-core/releases/) and unzip
2. Open a Chrome window and navigate to [chrome://extensions](chrome://extensions)
3. Enable **Developer mode** in the top right of the extension page
4. Hit **Load unpacked** and point it to the folder you just downloaded

Now you should see the Aptos wallet in your chrome extensions!

*Hint: Open your downloaded extensions by clicking the puzzle piece icon in your Chrome toolbar*

## Step 2) Wallet functionality
The wallet has implemented some of the basics of interacting with Aptos
- Create a new account
- Fund your account with test coins
- Send coins to another address
- Link to your account resources on Explorer

## Step 3) dApp Integration
Currently we have two requests a dApp webpage can make to the wallet:
- `account()`: gets the address of the account signed into the wallet
- `signTransaction(transaction)`: signs the given transaction and returns the signed transaction to be submitted to chain

### Usage
*Note: you will want to use our [typescript sdk](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) - `npm install aptos`*

```typescript
import { AptosAccount, AptosClient } from 'aptos'

// Gets the address of the account signed into the wallet
const accountAddress = await (window as any).aptos.account()

// Create a transaction
const client = new AptosClient('https://fullnode.devnet.aptoslabs.com')
const receiverAddress = '40b389daf74a98401f430ce8e73254dbffc5ce036ea32c33c15f14b858be3daf'
const amount = '1337'
const payload: Types.TransactionPayload = {
    type: 'script_function_payload',
    function: '0x1::TestCoin::transfer',
    type_arguments: [],
    arguments: [receiverAddress, amount]
}
const transaction = await client.generateTransaction(accountAddress, payload)

// Send transaction to the extension to be signed
const response = await (window as any).aptos.signTransaction(transaction)

// Submit signed transaction to chain
await client.submitTransaction(new AptosAccount(), response)
```

---
title: "Building the Wallet Extension"
slug: "building-wallet-extension"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Building the Wallet Extension

*Note: Wallet is very early in development and not secure or production ready*

This tutorial goes through how to build the wallet extension and how to use it with your dApp
1. Install the wallet on Chrome
2. Wallet functionality
3. dApp Integration

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
- View and create NFTs
- Select different networks

## Step 3) dApp Integration
dApps can make requests to the wallet from their website:
- `connect()`: prompts the user to allow connection from the dApp (*neccessary to make other requests*)
- `isConnected()`: returns if the dApp has established a connection with the wallet
- `account()`: gets the address of the account signed into the wallet
- `signAndSubmitTransaction(transaction)`: signs the given transaction and submits to chain
- `signTransaction(transaction)`: signs the given transaction and returns it to be submitted by the dApp
- `disconnect()`: Removes connection between dApp and wallet. Useful when the user wants to remove the connection.

### Usage

```typescript
// Establish connection to the wallet
const result = await (window as any).aptos.connect()

// Check connection status of wallet
const status = await (window as any).aptos.isConnected()

// Gets the address of the account signed into the wallet
const accountAddress = await (window as any).aptos.account()

// Create a transaction
const transaction = {
    type: 'script_function_payload',
    function: '0x1::Coin::transfer',
    type_arguments: ['0x1::TestCoin::TestCoin'],
    arguments: [receiverAddress, amount]
}

// Send transaction to the extension to be signed and submitted to chain
const response = await (window as any).aptos.signAndSubmitTransaction(transaction)

// Send transaction to the extension to be signed and returns
const signedTransaction = await (window as any).aptos.signTransaction(transaction)

// Disconnect dApp from the wallet
await (window as any).aptos.disconnect(transaction)
```

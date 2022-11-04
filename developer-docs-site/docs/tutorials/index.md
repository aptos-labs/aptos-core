---
title: "Developer Tutorials"
slug: "aptos-quickstarts"
---

# Developer Tutorials

If you are new to the Aptos blockchain, begin with these quickstarts before you get into in-depth development. These tutorials will help you become familiar with how to develop for the Aptos blockchain using the Aptos SDK.

Note, these tutorials should be used only in the [Aptos devnet environment](../nodes/aptos-deployments.md).

## Adjust network

For development purposes, the tutorials here assume you are working in the Aptos devnet network and therefore interacting with the devnet Aptos node and faucet service for creating and funding accounts. As noted in the relevant tutorials, these targets can be configured with the following environment variables once you have graduated to working in testnet and mainnet:
  * `APTOS_NODE_URL`
  * `APTOS_FAUCET_URL`

## Take tutorials

### [Your First Transaction](first-transaction.md)

How to [generate, submit and verify a transaction](first-transaction.md) to the Aptos blockchain. 

### [Your First NFT](your-first-nft.md)

Learn the Aptos `token` interface and how to use it to [generate your first NFT](your-first-nft.md). This interface is defined in the [`token.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) Move module.

### [Your First Move Module](first-move-module.md)

[Write your first Move module](first-move-module.md) for the Aptos blockchain. 

:::tip
Make sure to run the [Your First Transaction](first-transaction.md) tutorial before running your first Move module.
:::

### [Your First Dapp](first-dapp.md)

Learn how to [build your first dapp](first-dapp.md). Focuses on building the user interface for the dapp.

### [Your First Coin](first-coin.md)

Learn how to [deploy and manage a coin](first-coin.md). The `coin` interface is defined in the [`coin.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move) Move module.

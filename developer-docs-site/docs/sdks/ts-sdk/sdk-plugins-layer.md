---
title: "Plugins Layer"
slug: "typescript-sdk-plugins-layer"
---

A plugin is a component that can be added to the TypeScript SDK to extend or enhance its functionality. Plugins are meant to be built to support popular applications on the Aptos network and can be used to add new features, ease the use of the application operations and to customize the user experience.

## AptosToken class

The [AptosToken](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/aptos_token.ts) class is compatible with the [token V2 standard](../../standards/aptos-token-v2.md) and provides methods for creating and querying NFT collections and tokens.
It covers write methods that support creating, transferring, mutating, and burning tokens on-chain.

The main write methods supported by the AptosToken class are:

- Create Collection
- Mint
- Mint Soul Bound
- Burn Token
- Freeze Token Transfer
- Unfreeze Token Transfer
- Set Token Description
- Set Token Name
- Set Token URI
- Add Token Property
- Remove Token Property
- Update Token Property
- Add Types Property
- Update Types Property
- Transfer Token Ownership

## TokenClient class

The [TokenClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/token_client.ts) class is compatible with the [token V1 standard](../../standards/aptos-token.md) and provides methods for creating and querying the NFT collections and tokens.
It covers (1) write methods that support creating, transferring, mutating, and burning tokens on-chain and (2) read methods performing deserialization and returning data in TypeScript objects.

The main write methods supported by the TokenClient class are:

- Create Collection
- Create Token
- Offer Token
- Claim Token
- Directly Transfer Token
- Transfer Token with Opt-in
- Mutate Token Properties
- Burn Token by Owner or Creator

The main read methods deserializing on-chain data to TypeScript objects are:

- Get CollectionData
- Get TokenData
- Get Token of an Account

## FungibleAssetsClient class

The [FungibleAssetsClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/fungible_asset_client.ts) class is compatible with the [fungible asset component](../../standards/fungible-asset.md) and provides methods to transfer fungible assets between accounts and to check an account balance.

The main write methods are:

- Transfer
- Generate Transfer

The main read methods are:

- Get Primary Balance

## CoinClient class

The [CoinClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/coin_client.ts) class provides methods to interact with the coin module to transfer coins between accounts and to check an account balance. By default it transfers `0x1::aptos_coin::AptosCoin`, but you can specify a different coin type with the `coinType` argument.

The main methods are:

- Transfer
- Check Balance

## FaucetClient class

The [FaucetClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/faucet_client.ts) class is a thin wrapper for the Aptos faucet that provides a way to funds Aptos accounts. The class provides a request method to request token for an Aptos acount.

The main read methods are:

- Fund Account

## ANSClient class

The [ANSClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/ans_client.ts) class provides methods for creating an ANS name on the Aptos network and querying ANS data.
It covers (1) write methods that support creating a unique identity on the Aptos network by registering an Aptos name and (2) read methods that retrieve an account's ANS name using its address, as well as retrieving an account's address using its ANS name.

The main write methods supported by the SDK are:

- Mint an Aptos Name
- Init Reverse Lookup Registry

The main read methods are:

- Get Address By Name
- Get Primary Name By Address

## Build a Plugin

Developers can also create plugins to extend the functionality of the SDK and to provide users with a better experience. To do that, simply follow these steps:

1. Create a new `.ts` file under the `src/plugins/` folder and name it `<pluginName>.ts` (e.g. `ans_client`).
2. Create a class with the same `pluginName` (e.g. `AnsClient`) and implement it.
3. Export that file from the `src/plugins/index.ts` file (e.g. `export * from "./ans_client";`).

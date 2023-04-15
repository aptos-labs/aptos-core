---
title: "Typescript SDK Plugins Layer"
slug: "typescript-sdk-plugins-layer"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

### Token Client

The [TokenClient](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html) class provides methods for creating and querying the NFT collections and tokens.
It covers (1) write methods that support creating, transferring, mutating, and burning tokens on-chain and (2) read methods performing deserialization and returning data in TypeScript objects.

The main write methods supported by the Token SDK are:

- Create Collection
- Create Token
- Offer Token
- Claim Token
- Directly Transfer Token
- Transfer Token with Opt-in
- Mutate Token Properties
- Burn Token by Owner or Creator

The main read methods deserializing on-chain data to TS objects are:

- Get CollectionData
- Get TokenData
- Get Token of an Account

:::tip
You can also use this [TokenClient](https://aptos-labs.github.io/ts-sdk-doc/classes/TokenClient.html) API as an example of NFT Token API before you start developing your own application APIs with the SDK.
:::

### ANS Client

The [ANSClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/ans_client.ts) class provides methods for creating an ANS name on the Aptos network and querying ANS data.
It covers (1) write methods that support creating a unique identity on the Aptos network by registering an Aptos name and (2) read methods to get an account ANS name by an account address and get an account address by an ANS name.

The main write methods supported by the SDK are:

- mintAptosName

The main read methods are:

- getAddressByName
- getPrimaryNameByAddress

### Faucet Client

The [FaucetClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/faucet_client.ts) class is a thin wrapper for the Aptos faucet that provides a way to funds Aptos accounts. The class provides a request method to request token for an Aptos acount.

The main read methods are:

- fundAccount

### Coin Client

The [CoinClient](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/plugins/coin_client.ts) class provides method to intercat with the coin module to transfer coins between accounts and to check an account balance. By default it transfers `0x1::aptos_coin::AptosCoin`, but you can specify a different coin type with the `coinType` argument.
The main read methods are:

- transfer
- checkBalance

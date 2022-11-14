---
title: "Follow Aptos Token Standard"
slug: "index"
---

# Follow Aptos Token Standard

Token standards define how digital assets are created and used on their respective blockchains. These standards consist of rules to ensure your coins and tokens are compatible with the Aptos blockchain.

The documentation in this section comprises the Aptos Token Standard. See the pages and `.move` files below and also the [Move reference documentation](../../guides/move-guides/index.md#aptos-move-documentation) for the:

  * [Aptos Token Framework](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/overview.md)
  * [Aptos Framework](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/overview.md) containing `coin`

For digital assets, Aptos provides two Move modules:

## [Aptos Coin](aptos-coin)

The [`coin.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move) is a lightweight standard meant for simple, typesafe, and fungible assets. The coin standard is separated out into its own Move module to ensure that:
  - The coin standard can be used to create a token with an emphasis on simplicity and performance and with minimal metadata. 
  - The coin module remains a part of the Aptos core framework and can be used for currencies, for example the gas currency, thereby enhancing the core functionality of the Aptos framework.

See [Aptos Coin >](aptos-coin)

## [Aptos Token](aptos-token)

The [`token.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) Move module, on the other hand:

- Encapsulates rich, flexible assets, fungible and nonfungible, and collectibles. 
- The token standard is deployed as a separate package at the Aptos blockchain address `0x3`. 
- The token standard is designed to create an NFT or a semi-fungible or a fungible non-decimal token, with rich metadata and functionalities. A token definition of this type can be iterated rapidly to respond to the platform and user requirements. 

See [Aptos Token >](aptos-token)

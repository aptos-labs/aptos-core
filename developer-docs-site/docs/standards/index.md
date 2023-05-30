---
title: "Embrace the Aptos Standards"
---

# Aptos Standards

Standards define a common interoperable interface for all developers to build upon. They consist of rules to ensure compatibility across applications and wallets on the Aptos blockchain. See a [list of known coin resource addresses](https://github.com/hippospace/aptos-coin-list) in Aptos provided by 
hippospace.

## Object
### [Aptos Object](./aptos-object.md)

The [Object model](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/object.move) allows Move to represent a complex type as a set of resources stored within a single address and offers a rich capability model that allows for fine-grained resource control and ownership management.

See [Object >](./aptos-object.md)

## Digital Asset Standards
### [Aptos Coin](./aptos-coin.md)

The [Coin module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move) is a lightweight standard meant for simple, typesafe, and fungible assets. The coin standard is separated out into its own Move module to ensure that:
  - Applications and users can create and use simple tokens, with high performance and low gas overhead.
  - The Coin standard is part of the Aptos core framework so it can be used for currencies, including the gas currency.

See [Aptos Coin >](./aptos-coin.md)

### Aptos Token

#### [Token V2](./aptos-token-v2.md)
The new [Token v2 standard](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/token.move) allows:

- Rich, flexible assets and collectibles.
- Token v2 can be built up on easily to provide richer custom functionalities. An example of this is the [aptos_token module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/aptos_token.move)

The v2 standard is recommended for any new collections or protocols that want to build NFT or semi-fungible tokens.
See [Aptos Token v2 >](./aptos-token-v2.md)

#### [Token V1](./aptos-token.md)
The old existing [Token module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move), on the other hand:

- Encapsulates rich, flexible assets and collectibles. These assets are discrete (non-decimal) and can be fungible, semi-fungible, or nonfungible.
- The token standard is in its own `AptosToken` package at the Address `0x3` to allow for rapid iteration based on feedback from the community.

See [Aptos Token >](./aptos-token.md)

## Wallet standards
### [Aptos Wallet standards](./wallets.md)

The wallet standards ensure that all wallets use the same functionality for key features.  This includes:
- The same mnemonic so that wallets can be moved between providers.
- [Wallet adapter](../integration/wallet-adapter-concept.md) so that all applications can interact seamlessly with a common interface.

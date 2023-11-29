---
title: "Aptos Token Overview"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Aptos Token Standards

The [Aptos Digital Asset Standard](../../standards/digital-asset.md) defines the canonical Nonfungible Token on Aptos. Aptos leverages composability to extend the digital asset standard with features like fungibility via the [Fungible Asset standard](../../standards/fungible-asset.md). The concept of composability comes from the underlying data model for these constructs: the [Move object](../../standards/aptos-object.md) data model.

The rest of this document discusses how the Aptos token standards compare to the standards on Ethereum and Solana.

## Data models

To understand tokens, we begin by comparing the data models across different blockchains.

### Ethereum 

Ethereum has two types of accounts: 
* Externally-owned accounts which store a balance of Ether.
* Contract accounts which manage their underlying smart contracts and have an associated storage for persistent state, which can only be mutated by the associated contract.

In order to create a new NFT collection, a creator must deploy their own contract to the blockchain, which in turn will create a collection and set of NFTs within its storage.

### Solana

Unlike Ethereum or Aptos where data and code co-exist, Solana stores data and programs in separate accounts. There are two types of accounts on the Solana blockchain:
* Executable accounts only store contract code
* Non-executable accounts store data associated with and owned by executable accounts.

In order to create a new NFT collection, a creator calls an existing deployed program to populate a new collection and set of NFTs.

### Aptos

The [accounts](../../concepts/accounts.md) in Aptos store both smart contracts and data. Unlike Ethereum, the associated data of a smart contract is distributed across the space of all accounts in [resources](../../concepts/resources.md) within [accounts](../../concepts/accounts.md) or [objects](../../standards/aptos-object.md). For example, a collection and an NFT within that collection are stored in distinct objects at different addresses with the smart contract defining them at another address. A smart contract developer could also store data associated with the NFT and collection at the same address as the smart contract or in other objects.

There are two means to create NFTs on Aptos:

* The [no-code standard](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-22.md) allows creators to call into the contract to create new collections and tokens without deploying a new contract.
* Custom NFT contracts allow creators to customize their NFTs by extending the object model that can manage all aspects of their collection.

Aptos strikes a balance between the customizability offered by Ethereum with the simplicity of creating new collections like Solana.

Like Ethereum, Aptos requires indexing to determine the set of all NFTs owned by an account, while Solana has no need.

## Token standard comparison

The Fungible Token (FT) was initially introduced by [EIP-20](https://eips.ethereum.org/EIPS/eip-20), and Non-Fungible Token (NFT) was defined in [EIP-721](https://eips.ethereum.org/EIPS/eip-721). Later, [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155) combined FT and NFT or even Semi-Fungible Token (SFT) into one standard. 

The Ethereum token standards requires each token to deploy their own individual contract code to distinguish collection of tokens. Solana account model enables another pattern where code can be reused so that one generic program operates on various data. To create a new token, you could create an account that can mint tokens and more accounts that can receive them. The mint account itself uniquely determines the token type instead of contract account, and these are all passed as arguments to the one contract deployed to some executable account.

The collection of Aptos token standards shares some similarities with Solana, especially how it covers FT, NFT and SFT into a common on-chain code. Instead of deploying a new smart contract for each new token, a creator calls a function in the contract with the necessary arguments. Depending on which function you call, the token contract will mint/transfer/burn/... tokens.

### Token identification

Aptos identifies a token by its `Address` or `ObjectId`, a location within global storage. Collections are stored at a location determined by the address of the creator and the name of the collection.

In Ethereum, contracts are deployed on accounts determined by the account that is deploying the contract. NFTs are then stored as indexes into data tables within the contract.

In Solana, NFT data is stored under a mint account, independent of the program account.

### Token metadata

Aptos token has metadata in its `Token` resource with the data most commonly required by dapps to interact with tokens. Some examples include:
- `name`: The name of the token. It must be unique within a collection. 
- `description`: The description of the token.
- `uri`: A URL pointer to off-chain for more information about the token. The asset could be media such as an image or video or more metadata in a JSON file.
- `collection`: A pointer to the ObjectId of the collection.

Additional fields can be stored in creator-defined resources or the `PropertyMap` resource that defines a generalizable key-value map.

In Ethereum, only a small portion of such properties are defined as methods, such as `name()`, `symbol()`, `decimals()`, `totalSupply()` of ERC-20; or `name()` and `symbol()` and `tokenURI()` of the optional metadata extension for ERC-721; ERC-1155 also has a similar method `uri()` in its own optional metadata extension. Token metadata is not standardized so that dapps have to take special treatment case by case.

In Solana, the Token Metadata program offers a Metadata Account defining numerous metadata fields associated with a token as well, including `collection` which is defined in `TokenDataId` in Aptos. Solana, however, does not offer mutability for assets, unlike Aptos. Like Aptos, Token Metadata v1.1.0 offers an `attribute` container for customized properties.

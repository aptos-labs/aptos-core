---
title: "Aptos Token Standard Overview"
id: "aptos-token-overview"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Aptos Token Standard Overview

:::tip Aptos token standard
Also see the official token standard [Aptos Token](/concepts/coin-and-token/aptos-token.md).
:::

## Introduction
One of the major application of blockchain is token, which requires a standard defining
the pattern a token must comply with. The Aptos team and communities also have been moving
fast and steadily to support various projects and creators to launch their tokens on aptos
blockchain. As a new L1 blockchain equipped with Move, a relatively new language exclusively
designed for the safety of smart contract, our token standard inevitably differs from other
blockchains more or less. In this doc, we will briefly cover the comparison between aptos
ethereum and solana token standard.

### Account Model
In order to understand tokens, we have to compare the account model across different
blockchains. Here is a brief summary.

#### Ethereum 
Ethereum has two types of accounts. Externally-owned account (EOA) basically just stores
a balance of ETH. Contract account manages its underlying smart contract and has an associated 
storage map stores all the persisted data, which can only be mutated by the contract, enforced 
by EVM.

#### Solana
There are also two types of accounts on the Solana blockchain: executable and non-executable.
But how the data and code is stored and managed is different. In Ethereum, the data is stored
under the same contract account that manages this piece of data. In the Solana blockchain,
the data store and program are separated into different accounts. An executable account just
stores contract code whilst all the associated data are stored in non-executable accounts owned
by the executable account. 

#### Aptos
The accounts in Aptos are homogeneous in that they can both store smart contract and data. But
distinct from Ethereum, the associated data of a smart contract are distributed across the space
of all accounts. You can think the associated data of a contract in Ethereum is broken into pieces
stored in its owner's account. For example, a NFT smart contract will store all the NFTs and their
holders in the mapping and other metadata such like supply in a state variable, both belong to this
contract account. However, in Aptos, NFTs are stored under each owner's account, respectively. Other
token metadata will be kept under the creator account but not the account owning the contract code.
It is similar to Solana in the way that the contract code can be written to achieve maximum code
reuse instead of deploying similar code again and again just for domain separation. Meanwhile, the
access control mechanism and generics in Move natively enable this pattern and asset security.

Another interesting tradeoff between the account models of Ethereum and Solana/Aptos is data locality.
For Ethereum, it is hard to know all the tokens owned by a specific account but easy to get all the
owners of a token type. For Solana or Aptos, the opposite is true if no indexing is built. Ethereum
code can be easily written to do some operations like airdrop to all the holders relying on on-chain
data only while Solana/Aptos facilitate listing all the tokens an account owns.

### Token Standard Comparison
In Ethereum or even the whole blockchain world, Fungible Token (FT) was initially introduced by
[EIP-20](https://eips.ethereum.org/EIPS/eip-20) and Non-Fungible Token (NFT) was defined in 
[EIP-721](https://eips.ethereum.org/EIPS/eip-721). Later, [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155)
was proposed to combine FT and NFT or even Semi-Fungible Token (SFT) into the one standard. 

One defect of Ethereum token contract is each token have to deploy an individual contract code onto
a contract account to distinguish it from other tokens even it just differs by a name from another.
Solana's account model enables another pattern where code can be best reused that one generic program operates on
various data. To create a new token, you could create an account that can mint tokens, and more accounts that can
receive them. the mint account itself uniquely determines the token type instead of contract account, and these are all
passed as arguments to the one contract deployed to some executable account.

Aptos token standard shares some similarities with Solana, especially it covers FT, NFT and SFT in one standard and
also has similar generic token contract, which also implicitly defines token standard. Basically, instead of deploying
a new ERC20 smart contract for each new token, all you need to do is call a function in the contract with necessary
arguments. Depending on what function you call, the token contract will mint/tranfer/burn/... tokens.

#### Token Identification
Aptos identifies a token by its `TokenId` which includes `TokenDataId` and `property_version`. The
`property_version` shares the same concept with Edition Account in Solana but there is no explicit
counterpart in Ethereum as not required in any token standard interface.

`TokenDataId` is  a globally unique identifier of token group sharing all the metadata except for
`property_version`, including token creator address, collection name and token name. In Ethereum,
same concept is implemented by deploying a token contract under a unique address so a FT type or
a collection of NFTs is identified by different contract addresses. In Solana, the similar
concept for token identifier is implemented as mint account each of which will represent
one token type. While in Aptos, a creator account can have multiple token types created by giving
different collections and token names. 


#### Token Categorization
it is critical to understand, in Aptos, how to categorize different tokens to expect different sets
of features.
- `Fungible Token`: Each FT have one unique `TokenId`, which means tokens sharing the same creator, collection,
  name and property version are fungible.
- `Non-Fungible Token`: NFTs always have different `TokenId`s, but it is noted that NFTs belonging to the same
  collection (by nature also the same creator) will share the same `creator` and `collection`
  fields in their `TokenDataId`s.
- `Semi-Fungible Token`: The crypto communities lack of a common definition of SFT. The only consensus is
  SFTs are comparatively new types of tokens that combine the properties of NFTs and FTs.
  Usually it is realized via the customized logic based on different customized properties.

it's worth noting that Solana has an `Edition` concept that represents an NFT that was copied from a Master Edition NFT.
This can apply to use cases like tickets in that they are almost exactly the same except for some properties, for
example, serial numbers or seats for tickets. They could be implemented by bumping the token `property_version` and
mutating corresponding fields in `token_properties`.
In a nutshell, `Edition` is to Solana token is what `property_version` is to Aptos token but there is no similar concept
in Ethereum token standard.

#### Token Metadata
Aptos token has metadata defined in `TokenData` with the multiple fields that describe widely
acknowledged property data that needs to be understood by dapps. To name a few:
- `name`: The name of the token. It must be unique within a collection. 
- `description`: The description of the token.
- `uri`: A URL pointer to off-chain for more information about the token. The asset could be media such as an image or video or more metadata in a json file.
- `supply`: The total number of units of this Token.
- `token_properties`: a map-like structure containing optional or customized metadata not covered by existing
                        fields.

In Ethereum, only a small portion of such properties are defined as methods such as `name()`, `symbol()`,
`decimals()`, `totalSupply()` of ERC-20; or `name()` and `symbol()` and `tokenURI()` of the optional metadata extension
for ERC-721; ERC-1155 also has similar method `uri()` in its own optional metadata extension. Therefore, for token
on Ethereum, those token metadata is not standardized so that dapps have to take special treatment case by case,
which incurs unnecessary overhead for developers and users.

In Solana, Token Metadata program offers a Metadata Account defining numerous metadata fields associated
with a token as well, including `collection` which is defined in `TokenDataId` in Aptos. Unfortunately, it fails
to provide on-chain property with mutability config, which could improve the usability of token
standard by enabling more innovative smart contract logic based on on-chain properties. SFT is a good
example of this.  Instead, the `Token Standard` field introduced to Token Metadata v1.1.0 only provides `attributes`
as a container to hold customized properties. However, it is neither mutable nor on-chain, as an off-chain JSON
standard.


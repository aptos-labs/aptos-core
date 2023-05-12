---
title: "Compare Token Standards"
id: "aptos-token-comparison"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Compare Aptos Token Standards

:::tip Aptos token standard
For details, see the official [Aptos Token](../../standards/aptos-token.md) standard.
:::

One of the major applications of blockchains is token generation and use; this use requires a standard defining
the pattern a token must comply with. The Aptos team and communities have been moving
fast and steadily to support various projects and creators to launch their tokens on the Aptos
blockchain. As a new L1 blockchain equipped with the [Move programming language](../../move/move-on-aptos.md) - a relatively new language exclusively designed for the safety of smart contracts - our token standard inevitably differs from other
blockchains. In this document, we will briefly compare the token standard of Aptos with those of Ethereum and Solana.

## Account model
In order to understand tokens, we have to compare the account model across different
blockchains. This section contains a brief summary.

### Ethereum 
Ethereum has two types of accounts: Externally-owned accounts essentially just stores
a balance of Ether. Contract accounts manage their underlying smart contracts and have an associated 
storage map that stores all the persisted data, which can be mutated only by the contract, enforced 
by EVM.

### Solana
There are also two types of accounts on the Solana blockchain: executable and non-executable.
But how the data and code are stored and managed is different. In Ethereum, the data is stored
under the same contract account that manages this piece of data. In the Solana blockchain,
the data store and program are separated into different accounts. An executable account
stores only contract code while all the associated data is stored in non-executable accounts owned
by the executable account.

### Aptos
The [accounts](../../concepts/accounts.md) in Aptos are homogeneous in that they can store both smart contracts and data. But
distinct from Ethereum, the associated data of a smart contract is distributed across the space
of all accounts. The associated data of a contract in Ethereum is broken into pieces
stored in its owner's account. For example, an NFT smart contract will store all the NFTs and their
holders in the mapping and other metadata like supply in a state variable; both belong to this
contract account.

However, in Aptos, NFTs are stored under each owner's account, respectively. Other
token metadata will be kept under the creator account but not the account owning the contract code.
It is similar to Solana in the way that the contract code can be written to achieve maximum code
reuse instead of deploying similar code again and again just for domain separation. Meanwhile, the
access control mechanism and generics in Move natively enable this pattern and asset security.

Another interesting trade-off between the account models of Ethereum and Solana/Aptos is data locality.
For Ethereum, it is hard to know all the tokens owned by a specific account but easy to get all the
owners of a token type. For Solana and Aptos, the opposite is true if no indexing is built. Ethereum
code can be easily written to do some operations like airdrop to all the holders relying on on-chain
data only while Solana/Aptos facilitate listing all the tokens an account owns.

## Token standard comparison
In Ethereum or even the whole blockchain world, the Fungible Token (FT) was initially introduced by
[EIP-20](https://eips.ethereum.org/EIPS/eip-20), and Non-Fungible Token (NFT) was defined in 
[EIP-721](https://eips.ethereum.org/EIPS/eip-721). Later, [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155)
was proposed to combine FT and NFT or even Semi-Fungible Token (SFT) into the one standard. 

One deficiency of the Ethereum token contract is each token having to deploy individual contract code onto
a contract account to distinguish it from other tokens even if it simply differs by name.
Solana account model enables another pattern where code can be reused so that one generic program operates on
various data. To create a new token, you could create an account that can mint tokens and more accounts that can
receive them. The mint account itself uniquely determines the token type instead of contract account, and these are all
passed as arguments to the one contract deployed to some executable account.

The Aptos token standard shares some similarities with Solana, especially how it covers FT, NFT and SFT in one standard and
also has a similar generic token contract, which also implicitly defines token standard. Basically, instead of deploying
a new ERC20 smart contract for each new token, all you need to do is call a function in the contract with necessary
arguments. Depending on what function you call, the token contract will mint/transfer/burn/... tokens.

### Token identification
Aptos identifies a token by its `TokenId` that includes `TokenDataId` and `property_version`. The
`property_version` shares the same concept with *Edition Account* in Solana, but there is no explicit
counterpart in Ethereum as it is not required in any token standard interface.

`TokenDataId` is  a globally unique identifier of token group sharing all the metadata except for
`property_version`, including token creator address, collection name and token name. In Ethereum,
the same concept is implemented by deploying a token contract under a unique address so an FT type or
a collection of NFTs is identified by different contract addresses. In Solana, the similar
concept for token identifier is implemented as mint account, each of which will represent
one token type. In Aptos, a creator account can have multiple token types created by giving
different collections and token names. 


### Token categorization
it is critical to understand, in Aptos, how to categorize different tokens to expect different sets
of features:
- `Fungible Token`: Each FT has one unique `TokenId`, which means tokens sharing the same creator, collection,
  name and property version are fungible.
- `Non-Fungible Token`: NFTs always have different `TokenId`s, but it is noted that NFTs belonging to the same
  collection (by nature also the same creator) will share the same `creator` and `collection`
  fields in their `TokenDataId`s.
- `Semi-Fungible Token`: The crypto communities lack a common definition for SFT. The only consensus is
  SFTs are comparatively new types of tokens that combine the properties of NFTs and FTs.
  Usually this is realized via the customized logic based on different customized properties.

It's worth noting that Solana has an `Edition` concept that represents an NFT that was copied from a Master Edition NFT.
This can apply to use cases such as tickets in that they are almost exactly the same except for some properties, for
example, serial numbers or seats for tickets. They could be implemented in Aptos by bumping the token `property_version` and
mutating corresponding fields in `token_properties`.
In a nutshell, `Edition` is to Solana token is what `property_version` is to Aptos token; but there is no similar concept
in Ethereum token standard.

### Token metadata
Aptos token has metadata defined in `TokenData` with the multiple fields that describe widely
acknowledged property data that needs to be understood by dapps. To name a few fields:
- `name`: The name of the token. It must be unique within a collection. 
- `description`: The description of the token.
- `uri`: A URL pointer to off-chain for more information about the token. The asset could be media such as an image or video or more metadata in a JSON file.
- `supply`: The total number of units of this token.
- `token_properties`: a map-like structure containing optional or customized metadata not covered by existing fields.

In Ethereum, only a small portion of such properties are defined as methods, such as `name()`, `symbol()`,
`decimals()`, `totalSupply()` of ERC-20; or `name()` and `symbol()` and `tokenURI()` of the optional metadata extension
for ERC-721; ERC-1155 also has a similar method `uri()` in its own optional metadata extension. Therefore, for tokens
on Ethereum, that token metadata is not standardized so that dapps have to take special treatment case by case,
which incurs unnecessary overhead for developers and users.

In Solana, the Token Metadata program offers a Metadata Account defining numerous metadata fields associated
with a token as well, including `collection` which is defined in `TokenDataId` in Aptos. Unfortunately, it fails
to provide on-chain property with mutability configuration, which could improve the usability of the token
standard by enabling more innovative smart contract logic based on on-chain properties. SFT is a good
example of this. Instead, the `Token Standard` field introduced to Token Metadata v1.1.0 only provides `attributes`
as a container to hold customized properties. However, it is neither mutable nor on-chain, as an off-chain JSON
standard.

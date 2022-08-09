---
title: "Your First NFT"
slug: "your-first-nft"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

:::caution

The following tutorial is a work in progress. The Aptos (Non-Fungible) Token specification has not been formalized.

:::

# Your First NFT

An [NFT](https://en.wikipedia.org/wiki/Non-fungible_token) is a non-fungible token or data stored on a blockchain that uniquely defines ownership of an asset. NFTs were first defined in [EIP-721](https://eips.ethereum.org/EIPS/eip-721) and later expanded upon in [EIP-1155](https://eips.ethereum.org/EIPS/eip-1155). NFTs typically comprise of the following aspects:

- A name, the name of the asset, which must be unique within a collection
- A description, the description of the asset
- A URL, a non-descript pointer off-chain to more information about the asset could be media such as an image or video or more metadata
- A supply, the total number of units of this NFT, many NFTs have only a single supply while those that have more than one are referred to as editions

Additionally, most NFTs are part of a collection or a set of NFTs with a common attribute, e.g., theme, creator, or minimally contract. Each collection has a similar set of attributes:

- A name, the name of the collection, which must be unique within the creator's account
- A description, the description of the asset
- A URL, a non-descript pointer off-chain to more information about the asset could be media such as an image or video or more metadata

## Aptos digital asset token standard

The Aptos token standard is developed following the below principles:

- Provide a standard implementation to improve interoperability across ecosystem projects.

- Achieve maximal liquidity through defining NFT, Fungible (non-decimal) and Semi-Fungible tokens in one contract. These different types of tokens can be easily stored, transferred and transacted in the same way

- Enable the customization of token properties, users can define their own properties and store them on-chain.

- Reduce the cost of minting large amounts of NFT tokens. We support `lazy` on-chain mint through semi-fungible token

:::tip Aptos implementation for core NFT

The Aptos implementation for core NFTs or Tokens can be found in [Token.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move).

:::

## Aptos token definitions

### The Token Data model

<ThemedImage
  alt="Signed Transaction Flow"
  sources={{
    light: useBaseUrl('/img/docs/token-standard-light-mode.svg'),
    dark: useBaseUrl('/img/docs/token-standard-dark-mode.svg'),
  }}
/>

The token related data are stored at both creator’s account and owner’s account.

**Resource stored at creator’s address:**

| Field                      | Description                                                                                                                                                                                                                     |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Collections                | Maintains a table called `collection_data`, which maps the collection name to the `CollectionData`. It also stores all the TokenData that this creator creates.                                                                 |
| CollectionData             | Store the collection metadata. The `supply` is the number of tokens created for the current collection. `maxium` is the upper bound of tokens in this collection.                                                                |
| CollectionMutabilityConfig | Specify which field is mutable.                                                                                                                                                                                                 |
| TokenData                  | The main struct for holding the token metadata. Properties is a where user can add their own properties that are not defined in the token data. User can mint more tokens based on the TokenData and they share the same TokenData. |
| TokenMutabilityConfig      | Control which fields are mutable.                                                                                                                                                                                                |
| TokenDataId                | An id used for representing and querying TokenData on-chain. This id mainly contains 3 fields including creator address, collection name and token name.                                                                        |
| Royalty                    | Specify the denominator and numerator for calculating the royalty fee. It also has payee account address for depositing the Royalty.                                                                                            |
| PropertyValue              | Contains both value of a property and type of property.                                                                                                                                                                          |

**Resource stored at owner’s address:**

| Field      | Description                                                                                                                                                        |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| TokenStore | The main struct for storing the token owned by this address. It maps TokenId to the actual token.                                                                      |
| Token      | `amount` is the number of tokens.                                                                                                                                  |
| TokenId    | `TokenDataId` points to the metadata of this token. The `property_version` represents a token with mutated PropertyMap from `default_properties` in the TokenData. |

## Tokens tutorial

This tutorial will walk you through the process of:

- Creating your own token collection.
- Creating a token of our favorite cat.
- Giving that token to someone else.
- The on-chain lazy mint token through mutation.

This tutorial builds on [Your first transaction](/tutorials/your-first-transaction) as a library for this example. The following tutorial contains example code that can be downloaded in its entirety below:

<Tabs>
  <TabItem value="typescript" label="Typescript" default>

For this tutorial, will be focusing on `first_nft.ts` and re-using the `first_transaction.ts` library from the previous tutorial.

You can find the typescript project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript).

  </TabItem>
  <TabItem value="python" label="Python">

For this tutorial, will be focusing on `first_nft.py` and re-using the `first_transaction.py` library from the previous tutorial.

  </TabItem>
  <TabItem value="rust" label="Rust">
  Under construction.
  </TabItem>

</Tabs>

### Creating a Collection

The Aptos token enables creators to create collections. The `maximum` is the total number of tokens that can be created for this collection.

```rust
public(script) fun create_collection_script (
	creator: &signer,
	name: String,
	description: String,
	uri: String,
	maximum: u64,
	mutate_setting: vector<bool>,
)
```

These script functions can be called via the REST API. See below: 

<Tabs>
<TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_1
```

</TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/examples/python/first_nft.py section_1
```

  </TabItem>
  <TabItem value="rust" label="Rust">

Under construction.
</TabItem>
</Tabs>

### Creating a token

Tokens can be created after collection creation. To do so, the token must specify the same `collection` as specified as the name of a previously created collection `name`. The Move script function is:

```rust
public entry fun create_token_script(
	creator: &signer,
	collection: String,
	name: String,
	description: String,
	balance: u64,
	maximum: u64,
	uri: String,
	royalty_payee_address: address,
	royalty_points_denominator: u64,
	royalty_points_numerator: u64,
	token_mutate_setting: vector<bool>,
	property_keys: vector<String>,
	property_values: vector<vector<u8>>,
	property_types: vector<String>,
)
```

- The `balance` field is the initial amount to be created for this token.
- The `maximum` dictates the maximal number of tokens to be minted for this created `TokenData`.
- The `royalty_payee_address` is address that royalty is paid to.
- The quantity `royalty_points_numerator` / `royalty_points_denominator` is the percentage of sale price (`Royalty`) should be paid to the payee address. It can be a single owner's account address or an address of a shared account owned by a group of creators.
- The `token_mutate_setting` describes whether a field is `TokenData` is mutable.
- The `property_keys`, `property_values` and `property_types` are the property key value pairs that can be stored, read and write on-chain.

These script functions can be called via the REST API. See below:

<Tabs>
<TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_2
```

  </TabItem>
  <TabItem value="python" label="Python" >

```python
:!: static/examples/python/first_nft.py section_2
```

  </TabItem>
  <TabItem value="rust" label="Rust" >

Under construction.
</TabItem>
</Tabs>

### Giving away a token

In Aptos and Move, each token occupies space and has ownership. Because of this, token transfers are not unilateral and require two phase process similar to a bulletin board. The sender must first register that a token is available for the recipient to claim, the recipient must then claim this token. This is implemented in a proof of concept Move module called [`TokenTransfer`](https://github.com/aptos-labs/aptos-core/blob/nft/aptos-move/framework/aptos-framework/sources/TokenTransfers.move). 

The `SimpleToken` provides a few wrapper functions to support transferring to another account, claiming that transfer, or stopping that transfer.

#### Obtaining the token ID

In order to transfer the token, the sender must first identify the token id based upon knowing the creator's account, the collection name, and the token name. This can be obtained by querying REST:

<Tabs>
<TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_3
```

  </TabItem>
  <TabItem value="python" label="Python" >

```python
:!: static/examples/python/first_nft.py section_3
```

  </TabItem>
  <TabItem value="rust" label="Rust" >

Under construction.
</TabItem>
</Tabs>

#### Offering the token

The following Move script function in `Token` supports transferring a token to another account, effectively registering that the other account can claim the token:

```rust
public entry fun offer_script(
    sender: signer,
    receiver: address,
    creator: address,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64,
)
```

<Tabs>
<TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_4
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/examples/python/first_nft.py section_4
```

  </TabItem>
  <TabItem value="rust" label="Rust">

Under construction.
</TabItem>
</Tabs>

#### Claiming the token

The following Move script function in `SimpleToken` supports receiving a token provided by the previous function, effectively claiming a token:

```rust
public entry fun claim_script(
    receiver: signer,
    sender: address,
    creator: address,
    collection: String,
    name: String,
    property_version: u64,
)
```

<Tabs>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_nft.ts section_5
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/examples/python/first_nft.py section_5
```

  </TabItem>
  <TabItem value="rust" label="Rust">
Under construction.
  </TabItem>
</Tabs>

#### On-chain Lazy Mint

When Alice becomes a celebrity in her community, her cat NFTs are in high demand. However, Alice doesn't want to pay the cost of minting 10 million NFTs initially. She wants to only pay the cost when someone wants the NFT. She can mint 10 million uninitialized fungible cat token in one transaction.

When Jack wants to buy an NFT from Alice, she can mutate one fungible token.

```rust
    public entry fun mutate_token_properties(
        account: &signer,
        token_owner: address,
        creator: address,
        collection_name: String,
        token_name: String,
        token_property_version: u64,
        amount: u64,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    )
```

This will create a new `property_version` and create a new `TokenId` for the previous uninitialized fungible token (`property_version` = 0) to become an NFT. Alice can then transfer the NFT to Jack. Alice only need to pay the cost for creating NFT from the fungbile token when someone wants to buy.

<Tabs>
  <TabItem value="typescript" label="Typescript" default>
Under construction.
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/examples/python/first_nft.py section_6
```

  </TabItem>
  <TabItem value="rust" label="Rust">
Under construction.
  </TabItem>
</Tabs>

<!--- 
## Todos for Tokens

- Add ability for additional mints
- Add events -- needs feedback on what events
- Provide mutable APIs for tokens
- Provide testing tools for testing token related contracts
- Enable burning in a safe way
- Provide SDK, APIs and Indexing for tokens with better UX
- Add marketplace and auction contracts to our token standard
--->

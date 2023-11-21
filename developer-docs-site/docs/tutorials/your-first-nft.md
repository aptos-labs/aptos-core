---
title: "Your First NFT"
slug: "your-first-nft"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Your First NFT

This tutorial describes how to create and transfer non-fungible assets on the Aptos blockchain. The Aptos no-code implementation for non-fungible digital assets can be found in the [aptos_token.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/aptos_token.move) Move module.

## Step 1: Pick an SDK

Install your preferred SDK from the below list:

- [TypeScript SDK](../sdks/ts-sdk/v1/index.md)
- [Python SDK](../sdks/python-sdk.md)

---

## Step 2: Run the example

Each SDK provides an `examples` directory. This tutorial covers the `simple_aptos_token` example.

Clone the `aptos-core` repo:

```bash
git clone git@github.com:aptos-labs/aptos-core.git ~/aptos-core
```

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Navigate to the Typescript SDK examples directory:

```bash
cd ~/aptos-core/ecosystem/typescript/sdk/examples/typescript
```

Install the necessary dependencies:

```bash
pnpm install
```

Run the Typescript [`simple_aptos_token`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/simple_aptos_token.ts) example:

```bash
pnpm run simple_aptos_token
```

  </TabItem>
  <TabItem value="python" label="Python">

Navigate to the Python SDK directory:

```bash
cd ~/aptos-core/ecosystem/python/sdk
```

Install the necessary dependencies:

```bash
curl -sSL https://install.python-poetry.org | python3
poetry install
```

Run the Python [`simple_aptos_token`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple_aptos_token.py) example:

```bash
poetry run python -m examples.simple_aptos_token
```

  </TabItem>
</Tabs>

---

## Step 3: Understand the output

The following output should appear after executing the `simple_aptos_token` example, though some values will be different:

<Tabs groupId="sdk-output">
<TabItem value="typescript" label="Typescript">

```yaml
=== Addresses ===
Alice: 0x5acb91a64a2bbc5fc606a534709db5a1e60e439e15069d1e7bbaecddb4189b48
Bob: 0x612febb35dabc40df3260f7dd6c012f955671eb99862ba12390d2182ee3ab5de

=== Initial Coin Balances ===
Alice: 100000000
Bob: 100000000

=== Creating Collection and Token ===
Alice's collection: {
    "collection_id": "0x65b4000927646cae66251ed121f69ffa9acc2a6fb58a574fc66fd002b3d15d4f",
    "token_standard": "v2",
    "collection_name": "Alice's",
    "creator_address": "0x5acb91a64a2bbc5fc606a534709db5a1e60e439e15069d1e7bbaecddb4189b48",
    "current_supply": 1,
    "description": "Alice's simple collection",
    "uri": "https://alice.com"
}
Alice's token balance: 1
Alice's token data: {
    "token_data_id": "0xca2139c819fe03e2e268314c078948410dd14a64142ac270207a82cfddcc1fe7",
    "token_name": "Alice's first token",
    "token_uri": "https://aptos.dev/img/nyan.jpeg",
    "token_properties": {},
    "token_standard": "v2",
    "largest_property_version_v1": null,
    "maximum": null,
    "is_fungible_v2": false,
    "supply": 0,
    "last_transaction_version": 77174329,
    "last_transaction_timestamp": "2023-08-02T01:23:05.620127",
    "current_collection": {
        "collection_id": "0x65b4000927646cae66251ed121f69ffa9acc2a6fb58a574fc66fd002b3d15d4f",
        "collection_name": "Alice's",
        "creator_address": "0x5acb91a64a2bbc5fc606a534709db5a1e60e439e15069d1e7bbaecddb4189b48",
        "uri": "https://alice.com",
        "current_supply": 1
    }
}

=== Transferring the token to Bob ===
Alice's token balance: 0
Bob's token balance: 1

=== Transferring the token back to Alice ===
Alice's token balance: 1
Bob's token balance: 0

=== Checking if indexer devnet chainId same as fullnode chainId  ===
Fullnode chain id is: 67, indexer chain id is: 67

=== Getting Alices's NFTs ===
Alice current token ownership: 1. Should be 1

=== Getting Bob's NFTs ===
Bob current token ownership: 0. Should be 0
```

  </TabItem>
  <TabItem value="python" label="Python">

```yaml
=== Addresses ===
Alice: 0x391f8b07439768674023fb87ae5740e90fb8508600486d8ee9cc411b4365fe89
Bob: 0xfbca055c91d12989dc6a2c1a5e41ae7ba69a35454b04c69f03094bbccd5210b4

=== Initial Coin Balances ===
Alice: 100000000
Bob: 100000000

=== Creating Collection and Token ===

Collection data: {
    "address": "0x38f5310a8f6f3baef9a54daea8a356d807438d3cfe1880df563fb116731b671c",
    "creator": "0x391f8b07439768674023fb87ae5740e90fb8508600486d8ee9cc411b4365fe89",
    "name": "Alice's",
    "description": "Alice's simple collection",
    "uri": "https://aptos.dev"
}

Token owner: Alice
Token data: {
    "address": "0x57710a3887eaa7062f96967ebf966a83818017b8f3a8a613a09894d8465e7624",
    "owner": "0x391f8b07439768674023fb87ae5740e90fb8508600486d8ee9cc411b4365fe89",
    "collection": "0x38f5310a8f6f3baef9a54daea8a356d807438d3cfe1880df563fb116731b671c",
    "description": "Alice's simple token",
    "name": "Alice's first token",
    "uri": "https://aptos.dev/img/nyan.jpeg",
    "index": "1"
}

=== Transferring the token to Bob ===
Token owner: Bob

=== Transferring the token back to Alice ===
Token owner: Alice
```

  </TabItem>
</Tabs>

This example demonstrates:

- Initializing the REST and faucet clients.
- The creation of two accounts: Alice and Bob.
- The funding and creation of Alice and Bob's accounts.
- The creation of a collection and a token using Alice's account.
- Alice sending a token to Bob.
- Bob sending the token back to Alice.

---

## Step 4: The SDK in depth

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

:::tip See the full code
See [`simple_aptos_token`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/simple_aptos_token.ts) for the complete code as you follow the below steps.
:::
</TabItem>
<TabItem value="python" label="Python">

:::tip See the full code
See [`simple_aptos_token`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple_aptos_token.py) for the complete code as you follow the below steps.
:::
</TabItem>
</Tabs>

---

### Step 4.1: Initializing the clients

In the first step, the example initializes both the API and faucet clients.

- The API client interacts with the REST API.
- The faucet client interacts with the devnet Faucet service for creating and funding accounts.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_1a
```

Using the API client we can create a `TokenClient` that we use for common token operations such as creating collections and tokens, transferring them, claiming them, and so on.

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_1b
```

`common.ts` initializes the URL values as such:

```ts
:!: static/sdks/typescript/examples/typescript/common.ts section_1
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_1a
```

Using the API client we can create a `TokenClient` that we use for common token operations such as creating collections and tokens, transferring them, claiming them, and so on.

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_1b
```

[`common.py`](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk/examples/common.py) initializes these values as follows:

```python
:!: static/sdks/python/examples/common.py section_1
```

  </TabItem>
</Tabs>

:::tip

By default, the URLs for both the services point to Aptos devnet services. However, they can be configured with the following environment variables:

- `APTOS_NODE_URL`
- `APTOS_FAUCET_URL`
  :::

---

### Step 4.2: Creating local accounts

The next step is to create two accounts locally. [Accounts](../concepts/accounts.md) consist of a public address and the public/private key pair used to authenticate ownership of the account. This step demonstrates how to generate an Account and store its key pair and address in a variable.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_2
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_2
```

  </TabItem>
</Tabs>

:::info
Note that this only generates the local keypair. After generating the keypair and public address, the account still does not exist on-chain.
:::

---

### Step 4.3: Creating blockchain accounts

In order to actually instantiate the Account on-chain, it must be explicitly created somehow. On the devnet network, you can request free coins with the Faucet API to use for testing purposes. This example leverages the faucet to fund and inadvertently create Alice and Bob's accounts:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_3
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_3
```

  </TabItem>
</Tabs>

---

### Step 4.4: Creating a collection

Now begins the process of creating the digital, non-fungible assets. First, as the creator, you must create a collection that groups the assets. A collection can contain zero, one, or many distinct fungible or non-fungible assets within it. The collection is simply a container, intended only to group assets for a creator.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Your application will call `createCollection`:

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_4
```

This is the function signature of `createCollection`. It returns a transaction hash:

```ts
:!: static/sdks/typescript/src/plugins/aptos_token.ts createCollection
```

  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `create_collection`:

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_4
```

This is the function signature of `create_collection`. It returns a transaction hash:

```python
:!: static/sdks/python/aptos_sdk/aptos_token_client.py create_collection
```

  </TabItem>
</Tabs>

---

### Step 4.5: Creating a token

To create a token, the creator must specify an associated collection. A token must be associated with a collection, and that collection must have remaining tokens that can be minted. There are many attributes associated with a token, but the helper API exposes only the minimal amount required to create static content.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Your application will call `mint`:

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_5
```

This is the function signature of `mint`. It returns a transaction hash:

```ts
:!: static/sdks/typescript/src/plugins/aptos_token.ts mint
```

  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `mint_token`:

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_5
```

This is the function signature of `mint_token`. It returns a transaction hash:

```python
:!: static/sdks/python/aptos_sdk/aptos_token_client.py mint_token
```

  </TabItem>
</Tabs>

---

### Step 4.6: Reading token and collection metadata

Both the collection and token assets are [Objects](../standards/aptos-object) on-chain with unique addresses. Their metadata is stored at the object address. The SDKs provide convenience wrappers around querying this data:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

To read a collection's metadata:

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_6
```

To read a token's metadata:

```ts
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_8
```

Here's how `getTokenData` queries the token metadata using the [indexer client](../sdks/ts-sdk/v1/typescript-sdk-indexer-client-class):

```ts
:!: static/sdks/typescript/src/providers/indexer.ts getTokenData
```

  </TabItem>
  <TabItem value="python" label="Python">

To read a collection's metadata:

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_6
```

To read a token's metadata:

```python
:!: static/sdks/python/examples/simple_aptos_token.py get_token_data
```

  </TabItem>
</Tabs>

---

### Step 4.7: Reading an object's owner

Each object created from the `aptos_token.move` contract is a distinct asset. The assets owned by a user are stored separately from the user's account. To check if a user owns an object, check the object's owner:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts title="Extracting the balance from the indexer query"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_7
```

```ts title="Making the query to get the data"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts getTokenInfo
```

  </TabItem>
  <TabItem value="python" label="Python">

```python title="Get the object's resources and parse the owner"
:!: static/sdks/python/examples/simple_aptos_token.py section_7
```

```python title="How the owners dictionary is defined"
:!: static/sdks/python/examples/simple_aptos_token.py owners
```

  </TabItem>
</Tabs>

---

### Step 4.8: Transfer the object back and forth

Each object created from the `aptos_token.move` contract is a distinct asset. The assets owned by a user are stored separately from the user's account. To check if a user owns an object, check the object's owner:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts title="Extracting the balance from the indexer query"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_7
```

```ts title="Making the query to get the data"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts getTokenInfo
```

```ts title="Transfer the token from Alice to Bob"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_9
```

```ts title="Print each user's queried token amount"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_10
```

```ts title="Transfer the token back to Alice"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_11
```

```ts title="Print each user's queried token amount again"
:!: static/sdks/typescript/examples/typescript/simple_aptos_token.ts section_12
```

  </TabItem>
  <TabItem value="python" label="Python">

```python title="Transfer the token to Bob"
:!: static/sdks/python/examples/simple_aptos_token.py section_8
```

```python title="How the transfer_token function is defined"
:!: static/sdks/python/aptos_sdk/aptos_token_client.py transfer_token
```

```python title="Read the owner"
:!: static/sdks/python/examples/simple_aptos_token.py section_9
```

```python title="Transfer the token back to Alice"
:!: static/sdks/python/examples/simple_aptos_token.py section_10
```

```python title="Read the owner again"
:!: static/sdks/python/examples/simple_aptos_token.py section_11
```

  </TabItem>
</Tabs>

---

## Supporting documentation

- [Account basics](../concepts/accounts.md)
- [TypeScript SDK](../sdks/ts-sdk/v1/index.md)
- [Python SDK](../sdks/python-sdk.md)
- [REST API specification](https://aptos.dev/nodes/aptos-api-spec#/)

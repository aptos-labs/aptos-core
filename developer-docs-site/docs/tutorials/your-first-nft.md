---
title: "Your First NFT"
slug: "your-first-nft"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Your First NFT

This tutorial describes how to create and transfer NFTs on the Aptos blockchain. The Aptos implementation for core NFTs can be found in the [token.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/token.move) Move module.

For reference, also see:
* [mint_nft](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/mint_nft) Move example on how to airdrop an NFT 
* [mint_nft.rs](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/e2e-move-tests/src/tests/mint_nft.rs) Rust end-to-end test

## Step 1: Pick an SDK

Install your preferred SDK from the below list:

* [TypeScript SDK](../sdks/ts-sdk/index.md)
* [Python SDK](../sdks/python-sdk.md)
* [Rust SDK](../sdks/rust-sdk.md)

---

## Step 2: Run the example

Each SDK provides an `examples` directory. This tutorial covers the `simple-nft` example.

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

  Run the Typescript [`simple_nft`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/simple_nft.ts) example:
  ```bash
  pnpm run simple_nft
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

  Run the Python [`simple-nft`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple-nft.py) example:
  ```bash
  poetry run python -m examples.simple-nft
  ```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

## Step 3: Understand the output

The following output should appear after executing the `simple-nft` example, though some values will be different:

```yaml
=== Addresses ===
Alice: 0xeef95e86c160fa10a71675c6075f44f8f2c6125f57b4b589424f1fbee385f754
Bob: 0x4dcd7b180c123fdb989d10f71fba6c978bda268c2e3660c169bdb55f67aab776

=== Initial Coin Balances ===
Alice: 100000000
Bob: 100000000

=== Creating Collection and Token ===
Alice's collection: {
    "description": "Alice's simple collection",
    "maximum": "18446744073709551615",
    "mutability_config": {
        "description": false,
        "maximum": false,
        "uri": false
    },
    "name": "Alice's",
    "supply": "1",
    "uri": "https://alice.com"
}
Alice's token balance: 1
Alice's token data: {
    "default_properties": {
        "map": {
            "data": []
        }
    },
    "description": "Alice's simple token",
    "largest_property_version": "0",
    "maximum": "18446744073709551615",
    "mutability_config": {
        "description": false,
        "maximum": false,
        "properties": false,
        "royalty": false,
        "uri": false
    },
    "name": "Alice's first token",
    "royalty": {
        "payee_address": "0xeef95e86c160fa10a71675c6075f44f8f2c6125f57b4b589424f1fbee385f754",
        "royalty_points_denominator": "0",
        "royalty_points_numerator": "0"
    },
    "supply": "1",
    "uri": "https://aptos.dev/img/nyan.jpeg"
}

=== Transferring the token to Bob ===
Alice's token balance: 0
Bob's token balance: 1

=== Transferring the token back to Alice using MultiAgent ===
Alice's token balance: 1
Bob's token balance: 0
```

This example demonstrates:

* Initializing the REST and faucet clients.
* The creation of two accounts: Alice and Bob.
* The funding and creation of Alice and Bob's accounts.
* The creation of a collection and a token using Alice's account.
* Alice offering a token and Bob claiming it.
* Bob unilaterally sending the token to Alice via a multiagent transaction.

---

## Step 4: The SDK in depth

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

:::tip See the full code
See [`simple_nft`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/simple_nft.ts) for the complete code as you follow the below steps.
:::
  </TabItem>
  <TabItem value="python" label="Python">

:::tip See the full code
See [`simple-nft`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple-nft.py) for the complete code as you follow the below steps.
:::
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
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
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_1a
```

Using the API client we can create a `TokenClient` that we use for common token operations such as creating collections and tokens, transferring them, claiming them, and so on.
```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_1b
```

`common.ts` initializes the URL values as such:
```ts
:!: static/sdks/typescript/examples/typescript/common.ts section_1
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_1
```

[`common.py`](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk/examples/common.py) initializes these values as follows:

```python
:!: static/sdks/python/examples/common.py section_1
```
  </TabItem>
  <TabItem value="rust" label="Rust">


```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_1a
```

Using the API client we can create a `CoinClient` that we use for common coin operations such as transferring coins and checking balances.
```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_1b
```

In the example we initialize the URL values as such:
```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_1c
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

The next step is to create two accounts locally. [Accounts](../concepts/accounts.md) represent both on and off-chain state. Off-chain state consists of an address and the public/private key pair used to authenticate ownership. This step demonstrates how to generate that off-chain state.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_2
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_2
```
  </TabItem>
  <TabItem value="rust" label="Rust">


```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_2
```
  </TabItem>
</Tabs>

---

### Step 4.3: Creating blockchain accounts

In Aptos, each account must have an on-chain representation in order to receive tokens and coins and interact with other dApps. An account represents a medium for storing assets; hence, it must be explicitly created. This example leverages the Faucet to create Alice and Bob's accounts:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_3
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_3
```
  </TabItem>
  <TabItem value="rust" label="Rust">
  
  Since the Rust example here uses the same `transfer-coin.rs` function as in the [Your First Transaction](first-transaction.md) tutorial, it creates but does not fund Bob's account.

```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_3
```
  </TabItem>
</Tabs>

---

### Step 4.4: Creating a collection

Now begins the process of creating tokens. First, the creator must create a collection to store tokens. A collection can contain zero, one, or many distinct tokens within it. The collection does not restrict the attributes of the tokens, as it is only a container.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Your application will call `createCollection`:
```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_4
```

The is the function signature of `createCollection`. It returns a transaction hash:
```ts
:!: static/sdks/typescript/src/plugins/token_client.ts createCollection
```
  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `create_collection`:
```python
:!: static/sdks/python/examples/simple-nft.py section_4
```

This is the function signature of `create_collection`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/async_client.py create_collection
```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.5: Creating a token

To create a token, the creator must specify an associated collection. A token must be associated with a collection, and that collection must have remaining tokens that can be minted. There are many attributes associated with a token, but the helper API exposes only the minimal amount required to create static content.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Your application will call `createToken`:
```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_5
```

The is the function signature of `createToken`. It returns a transaction hash:
```ts
:!: static/sdks/typescript/src/plugins/token_client.ts createToken
```
  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `create_token`:
```python
:!: static/sdks/python/examples/simple-nft.py section_5
```

The is the function signature of `create_token`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/async_client.py create_token
```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.6: Reading token and collection metadata

Both the collection and token metadata are stored on the creator's account within their `Collections` in a table. The SDKs provide convenience wrappers around querying these specific tables:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

To read a collection's metadata:
```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_6
```

To read a token's metadata:
```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_8
```

Here's how `getTokenData` queries the token metadata:
```ts
:!: static/sdks/typescript/src/plugins/token_client.ts getTokenData
```

  </TabItem>
  <TabItem value="python" label="Python">

To read a collection's metadata:
```python
:!: static/sdks/python/examples/simple-nft.py section_6
```

To read a token's metadata:
```python
:!: static/sdks/python/examples/simple-nft.py section_8
```

Here's how `get_token_data` queries the token metadata:
```python
:!: static/sdks/python/aptos_sdk/async_client.py read_token_data_table
```

  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.7: Reading a token balance

Each token within Aptos is a distinct asset. The assets owned by the user are stored within their `TokenStore`. To get the balance:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_7
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_7
```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.8: Offering and claiming a token

Many users of other blockchains have received unwanted tokens that may cause anything from minimal embarrassment to serious ramifications. Aptos gives the rights to each account owner to dictate whether or not to receive unilateral transfers. By default, unilateral transfers are unsupported. So Aptos provides a framework for *offering* and *claiming* tokens.

To offer a token:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_9
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_9
```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon!
  </TabItem>
</Tabs>

To claim a token:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_10
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_10
```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.9: Safe unilateral transferring of a token

To support safe unilateral transfers of a token, the sender may first ask the recipient to acknowledge off-chain a pending transfer. This comes in the form of a multiagent transaction request. Multiagent transactions contain multiple signatures, one for each on-chain account. Move then can leverage this to give `signer`-level permissions to all who signed the transaction. For token transfers, this process ensures the receiving party does indeed want to receive this token without requiring the use of the token transfer framework described above.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_11
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_11
```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

## Supporting documentation

* [Account basics](../concepts/accounts.md)
* [TypeScript SDK](../sdks/ts-sdk/index.md)
* [Python SDK](../sdks/python-sdk.md)
* [Rust SDK](../sdks/rust-sdk.md)
* [REST API specification](https://aptos.dev/nodes/aptos-api-spec#/)

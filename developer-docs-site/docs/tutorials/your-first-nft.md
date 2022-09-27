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

## Step 1: Pick an SDK

Install your preferred SDK from the below list:

* [Typescript SDK][typescript-sdk]
* [Python SDK][python-sdk]
* [Rust SDK][rust-sdk]

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
  yarn install
  ```

  Run the Typescript [`simple_nft`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/simple_nft.ts) example:
  ```bash
  yarn run simple_nft
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
  poetry update
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
Alice: 0x9df0f527f3a0b445e4d5c320cfa269cdefafc7cd1ed17ffce4b3fd485b17aafb
Bob: 0xfcc74af84dde26b0050dce35d6b3d11c60f5c8c58728ca3a0b11035942a0b1de

=== Initial Coin Balances ===
Alice: 20000
Bob: 20000

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
    "uri": "https://aptos.dev"
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
    "maximum": "1",
    "mutability_config": {
        "description": false,
        "maximum": false,
        "properties": false,
        "royalty": false,
        "uri": false
    },
    "name": "Alice's first token",
    "royalty": {
        "payee_address": "0x9df0f527f3a0b445e4d5c320cfa269cdefafc7cd1ed17ffce4b3fd485b17aafb",
        "royalty_points_denominator": "1000000",
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

In the first step the example initializes both the API and faucet clients.

- The API client interacts with the REST API, and
- The faucet client interacts with the devnet Faucet service for creating and funding accounts.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_1a
```

Using the API client we can create a `TokenClient`, which we use for common token operations such as creating collections and tokens, transferring them, claiming them, and so on.
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

Coming soon.
  </TabItem>
</Tabs>


:::tip

By default the URLs for both the services point to Aptos devnet services. However, they can be configured with the following environment variables:
  - `APTOS_NODE_URL`
  - `APTOS_FAUCET_URL`
:::

---

### Step 4.2: Creating local accounts

The next step is to create two accounts locally. [Accounts][account_basics] represent both on and off-chain state. Off-chain state consists of an address and the public, private key pair used to authenticate ownership. This step demonstrates how to generate that off-chain state.

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

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.3: Creating blockchain accounts

In Aptos, each account must have an on-chain representation in order to support receive tokens and coins as well as interacting in other dApps. An account represents a medium for storing assets, hence it must be explicitly created. This example leverages the Faucet to create Alice and Bob's accounts:

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

Coming soon.
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

The function signature of `createCollection`. It returns a transaction hash:
```ts
:!: static/sdks/typescript/src/token_client.ts createCollection
```
  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `create_collection`:
```python
:!: static/sdks/python/examples/simple-nft.py section_4
```

The function signature of `create_collection`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/client.py create_collection
```
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.5: Creating a token

To create a token, the creator must specify an associated collection. A token must be associated with a collection and that collection must have remaining tokens that can be minted. There are many attributes associated with a token, but the helper API only exposes the minimal amount required to create static content.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Your application will call `createToken`:
```ts
:!: static/sdks/typescript/examples/typescript/simple_nft.ts section_5
```

The function signature of `createToken`. It returns a transaction hash:
```ts
:!: static/sdks/typescript/src/token_client.ts createToken
```
  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `create_token`:
```python
:!: static/sdks/python/examples/simple-nft.py section_5
```

The function signature of `create_token`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/client.py create_token
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
:!: static/sdks/typescript/src/token_client.ts getTokenData
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
:!: static/sdks/python/aptos_sdk/client.py read_token_data_table
```

  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
</Tabs>

---

### Step 4.7: Reading a token balance

Each token within Aptos is a distinct asset, the assets owned by the user are stored within their `TokenStore`. To get the balance:

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

Many users have received unwanted tokens that may cause minimally embarrassment to serious ramifications. Aptos gives the rights to each owner of an account to dictate whether or not to receive unilateral transfers. By default, unilateral transfers are unsupported. So Aptos provides a framework for *offering* and *claiming* tokens.

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

To support safe unilateral transfers of a token, the sender may first ask the recipient to acknowledge off-chain about a pending transfer. This comes in the form of a multiagent transaction request. Multiagent transactions contain multiple signatures, one for each on-chain account. Move then can leverage this to give `signer` level permissions to all that signed. For token transfers, this ensures that the receiving party does indeed desire to receive this token without requiring the use of the token transfer framework described above.

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

### Step 4.10: Enabling unilateral token transfers

Coming soon.

<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

Coming soon.
  </TabItem>
  <TabItem value="rust" label="Rust">

Coming soon.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

Coming soon.
  </TabItem>
</Tabs>

[account_basics]: /concepts/basics-accounts
[typescript-sdk]: /sdks/ts-sdk/index
[python-sdk]: /sdks/python-sdk
[rust-sdk]: /sdks/rust-sdk
[rest_spec]: https://fullnode.devnet.aptoslabs.com/v1/spec#/

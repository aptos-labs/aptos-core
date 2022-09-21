---
title: "Your First Transaction"
slug: "your-first-transaction"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Transaction

This tutorial describes how to generate and submit transactions to the Aptos blockchain, and verify these submitted transactions. The `transfer-coin` example used in this tutorial is built with the Aptos SDKs.

## Step 1: Pick an SDK

Install your preferred SDK from the below list:

* [Typescript SDK][typescript-sdk]
* [Python SDK][python-sdk]
* [Rust SDK][rust-sdk]

---

## Step 2: Run the example

Clone the `aptos-core` repo:
```bash
git clone https://github.com/aptos-labs/aptos-core.git
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

  Run the [`transfer_coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/transfer_coin.ts) example:

  ```bash
  yarn run transfer_coin
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

  Run the [`transfer-coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/transfer-coin.py) example:
  ```bash
  poetry run python -m examples.transfer-coin
  ```
  </TabItem>
  <TabItem value="rust" label="Rust">

  Navigate to the Rust SDK directory:
  ```bash
  cd ~/aptos-core/sdk
  ```

  Run the [`transfer-coin`](https://github.com/aptos-labs/aptos-core/blob/main/sdk/examples/transfer-coin.rs) example:
  ```bash
  cargo run --example transfer-coin
  ```
  </TabItem>
</Tabs>

---

## Step 3: Understand the output

An output very similar to the following will appear after executing the above command:

```yaml
=== Addresses ===
Alice: 0x0baec07bfc42f8018ea304ddc307a359c1c6ab20fbce598065b6cb19acff7043
Bob: 0xc98ceafadaa32e50d06d181842406dbbf518b6586ab67cfa2b736aaddeb7c74f

=== Initial Balances ===
Alice: 20000
Bob: 0

=== Intermediate Balances ===
Alice: 18996
Bob: 1000

=== Final Balances ===
Alice: 17992
Bob: 2000
```

The above output demonstrates that the `transfer-coin` example executes the following steps:

* Initializing the REST and faucet clients.
* The creation of two accounts: Alice and Bob.
  * The funding and creation of Alice's account from a faucet.
  * The creation of Bob's account from a faucet.
* The transferring of 1000 coins from Alice to Bob.
* The 4 coins of gas paid for by Alice to make that transfer.
* Another transfer of 1000 coins from Alice to Bob.
* The additional 4 coins of gas paid for by Alice to make that transfer.

Next, see below a walk-through of the SDK functions that are used to accomplish the above steps.

---

## Step 4: The SDK in depth

The `transfer-coin` example code uses helper functions to interact with the [REST API][rest_spec]. This section reviews each of the calls and gives insights into functionality.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

:::tip See the full code
See the Typescript [`transfer-coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/examples/typescript/transfer_coin.ts) for the complete code as you follow the below steps.
:::
  </TabItem>
  <TabItem value="python" label="Python">

:::tip See the full code
See the Python [`transfer-coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/transfer-coin.py) for the complete code as you follow the below steps.
:::
  </TabItem>
  <TabItem value="rust" label="Rust">

:::tip See the full code
See the Rust [`transfer-coin`](https://github.com/aptos-labs/aptos-core/blob/main/sdk/examples/transfer-coin.rs) for the complete code as you follow the below steps.
:::
  </TabItem>
</Tabs>

---

### Step 4.1: Initializing the clients

In the first step, the `transfer-coin` example initializes both the REST and faucet clients.

- The REST client interacts with the REST API, and
- The faucet client interacts with the devnet Faucet service for creating and funding accounts.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/transfer_coin.ts section_1
```

Using the API client we can create a `CoinClient`, which we use for common coin operations such as transferring coins and checking balances.
```ts
:!: static/sdks/typescript/examples/typescript/transfer_coin.ts section_1a
```

`common.ts` initializes the URL values as such:
```ts
:!: static/sdks/typescript/examples/typescript/common.ts section_1
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_1
```

The [`common.py`](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk/examples/common.py) initializes these values as follows:

```python
:!: static/sdks/python/examples/common.py section_1
```
  </TabItem>
  <TabItem value="rust" label="Rust">

```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_1a
```

Using the API client we can create a `CoinClient`, which we use for common coin operations such as transferring coins and checking balances.
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
:!: static/sdks/typescript/examples/typescript/transfer_coin.ts section_2
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_2
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

In Aptos, each account must have an on-chain representation in order to support receive tokens and coins as well as interacting in other dApps. An account represents a medium for storing assets, hence it must be explicitly created. This example leverages the Faucet to create and fund Alice's account and to only create Bob's account:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/transfer_coin.ts section_3
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_3
```
  </TabItem>
  <TabItem value="rust" label="Rust">

```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_3
```
  </TabItem>
</Tabs>

---

### Step 4.4: Reading balances

In this step, the SDK translates a single call into the process of querying a resource and reading a field from that resource.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/transfer_coin.ts section_4
```

Behind the scenes, the `checkBalance` function in `CoinClient` in the SDK queries the CoinStore resource for the AptosCoin and reads the current stored value:

```ts
:!: static/sdks/typescript/src/coin_client.ts checkBalance
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_4
```

Behind the scenes, the SDK queries the CoinStore resource for the AptosCoin and reads the current stored value:
```python
def account_balance(self, account_address: str) -> int:
    """Returns the test coin balance associated with the account"""
    return self.account_resource(
        account_address, "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"
    )["data"]["coin"]["value"]
```
  </TabItem>
  <TabItem value="rust" label="Rust">

```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_4
```

Behind the scenes, the SDK queries the CoinStore resource for the AptosCoin and reads the current stored value:
```rust
let balance = self
    .get_account_resource(address, "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>")
    .await?;
```
  </TabItem>
</Tabs>

---

### Step 4.5: Transferring

Like the previous step, this is another helper step that constructs a transaction which transfers the coins from Alice to Bob. For correctly generated transactions, the API will return a transaction hash that can be used in the subsequent step to check on the transaction status. The Aptos blockchain does perform a handful of validation checks on submission and if any of those fail, the user will instead be given an error. These validations include the transaction signature, unused sequence number, and submitting the transaction to the appropriate chain.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
:!: static/sdks/typescript/examples/typescript/transfer_coin.ts section_5
```

Behind the scenes, the `transfer` function generates a transaction payload and has the client sign, send, and wait for it:
```ts
:!: static/sdks/typescript/src/coin_client.ts transfer
```

Within the client, <code>generateSignSubmitTransaction</code> is doing this:
```ts
:!: static/sdks/typescript/src/aptos_client.ts generateSignSubmitTransactionInner
```

Breaking the above down into pieces:
1. `transfer` internally is a `EntryFunction` in the [Coin Move module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move#L412), i.e. an entry function in Move that is directly callable.
1. The Move function is stored on the coin module: `0x1::coin`.
1. Because the Coin module can be used by other coins, the transfer must explicitly specify which coin type to transfer. If not specified with `coinType` it defaults to `0x1::aptos_coin::AptosCoin`.


  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_5
```

Behind the scenes the Python SDK generates, signs, and submits a transaction:
```python
:!: static/sdks/python/aptos_sdk/client.py bcs_transfer
```

Breaking the above down into pieces:
1. `transfer` internally is a `EntryFunction` in the [Coin Move module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move#L412), i.e. an entry function in Move that is directly callable.
1. The Move function is stored on the coin module: `0x1::coin`.
1. Because the Coin module can be used by other coins, the transfer must explicitly use a `TypeTag` to define which coin to transfer.
1. The transaction arguments must be placed into `TransactionArgument`s with type specifiers (`Serializer.{type}`), that will serialize the value into the appropriate type at transaction generation time.


  </TabItem>
  <TabItem value="rust" label="Rust">

```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_5
```

Behind the scenes the Rust SDK generates, signs, and submits a transaction:
```rust
:!: static/sdks/rust/src/coin_client.rs section_1
```

Breaking the above down into pieces:
1. First, we fetch the chain ID, necessary for building the transaction payload.
1. `transfer` internally is a `EntryFunction` in the [Coin Move module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move#L412), i.e. an entry function in Move that is directly callable.
1. The Move function is stored on the coin module: `0x1::coin`.
1. Because the Coin module can be used by other coins, the transfer must explicitly use a `TypeTag` to define which coin to transfer.
1. The transaction arguments, such as `to_account` and `amount`, must be encoded as BCS to use with the `TransactionBuilder`.


  </TabItem>
</Tabs>

---

### Step 4.6: Waiting for transaction resolution

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

In Typescript, just calling `coinClient.transfer` is sufficient to wait for the transaction to complete. The function will return the `Transaction` returned by the API once it is processed (either successfully or unsuccessfully) or throw an error if processing time exceeds the timeout.

You can set `checkSuccess` to true when calling `transfer` if you'd like it to throw if the transaction was not committed successfully:
```ts
:!: static/sdks/typescript/examples/typescript/transfer_coin.ts section_6a
```

  </TabItem>
  <TabItem value="python" label="Python">

The transaction hash can be used to query the status of a transaction:

```python
:!: static/sdks/python/examples/transfer-coin.py section_6
```
  </TabItem>
  <TabItem value="rust" label="Rust">

The transaction hash can be used to query the status of a transaction:

```rust
:!: static/sdks/rust/examples/transfer-coin.rs section_6
```
  </TabItem>
</Tabs>

[account_basics]: /concepts/basics-accounts
[typescript-sdk]: /sdks/ts-sdk/index
[python-sdk]: /sdks/python-sdk
[rust-sdk]: /sdks/rust-sdk
[rest_spec]: https://fullnode.devnet.aptoslabs.com/v1/spec#/

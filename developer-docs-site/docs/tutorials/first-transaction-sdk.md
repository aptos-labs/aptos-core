---
title: "Your First Transaction using the SDK"
slug: "your-first-transaction-sdk"
sidebar_position: 0
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Transaction

This tutorial introduces the Aptos SDKs and how to generate, submit, and verify transactions submitted to the Aptos Blockchain.

## Step 1: Pick an SDK

* [Official Aptos Python SDK][python-sdk]
* Official Aptos Typescript SDK -- TBA
* Official Aptos Rust SDK -- TBA

## Step 2: Run the Example

Each SDK provides an examples directory. This tutorial covers the `transfer-coin` example.

<Tabs>
  <TabItem value="python" label="Python">

In the SDK directory run: `python -m examples.transfer-coin`
  </TabItem>
  <TabItem value="rust" label="Rust">

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

## Step 3: Understand the Output

The following output should appear after executing the `transfer-coin` example, though some values will be different:

```
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

This example demonstrates:

* Initializing the REST and Faucet clients
* The creation of two accounts: Alice and Bob
* The funding and creation of Alice's account from a faucet
* The creation of Bob's account from a faucet
* The transferring of 1000 coins from Alice to Bob
* The 4 coins for gas paid for by Alice to make that tansfer
* Another transferring of 1000 coins from Alice to Bob
* The additional 4 coins of gas paid for by Alice to make that transfer

## Step 4: The SDK in Depth

The example file leverages helper functions to interact with the [REST API][rest_spec]. This section reviews each of the calls and gives insights into functionality.

### Step 4.1: Initializing the Clients

In the first step, the example initializes both the REST and Faucet clients. The REST client interacts with the REST API, whereas the Faucet client interacts with the devnet Faucet service for creating and funding accounts.

<Tabs>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_1
```

`common.py` initializes these values as such:
```python
:!: static/sdks/python/examples/common.py section_1
```
  </TabItem>
  <TabItem value="rust" label="Rust">

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

:::tip

The URLs for both services by default, point to our devnet services. However, they can be configured with the following environment variables: `APTOS_NODE_URL` and `APTOS_FAUCET_URL`.
:::

### Step 4.2: Creating local accounts

The next step is to create two accounts from the locally. [Accounts][account_basics] represent both on and off-chain state. Off-chain state consists of an address and the public, private key pair used to authenticate ownership. This step demonstrates how to generate that off-chain state.

<Tabs>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_2
```
  </TabItem>
  <TabItem value="rust" label="Rust">

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

### Step 4.3: Creating blockchain accounts

In Aptos, each account must have an on-chain representation in order to support receive tokens and coins as well as interacting in other dApps. An account represents a medium for storing assets, hence it must be explicitly created. This example leverages the Faucet to create and fund Alice's account and to only create Bob's:

<Tabs>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_3
```
  </TabItem>
  <TabItem value="rust" label="Rust">

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

### Step 4.4: Reading balances

In this step, the SDK translates a single call into the process of querying a resource and reading a field from that resource.

<Tabs>
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

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

### Step 4.5: Transferring

Like the previous step, this is another helper step that constructs a transaction that transfers the coins from Alice to Bob. For correctly generated transactions, the API will return a transaction hash that can be used in the ensuing step to check on the transaction status. Aptos does perform a handful of validation checks on submission and if any of those fail, the user will instead be given an error. These validations include the transaction signature, unused sequence number, and submitting the transaction to the appropriate chain.

<Tabs>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_5
```

Behind the scenes, the SDK generates, signs, and submits a transaction:
```python
:!: static/sdks/python/aptos_sdk/client.py bcs_transfer
```

Breaking the above down into pieces:<br/>
(1) `transfer` internally is a `EntryFunction` or an entry function in Move that is directly callable.<br/>
(2) The Move function is stored on the coin module: `0x1::coin`.<br/>
(3) Because the Coin module can be used by other coins, the transfer must explicitly use a `TypeTag` to define which coin to transfer.<br/>
(4) The transaction arguments must be placed into `TransactionArgument`s with type specifiers (`Serializer.{type}`), that will serialize the value into the appropriate type at transaction generation time.

  </TabItem>
  <TabItem value="rust" label="Rust">

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

### Step 4.6: Waiting for transaction resolution

The transaction hash can be used to query the status of a transaction:

<Tabs>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_6
```
  </TabItem>
  <TabItem value="rust" label="Rust">

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

[account_basics]: /concepts/basics-accounts
[python-sdk]: /sdks/python-sdk
[rest_spec]: https://fullnode.devnet.aptoslabs.com/v1/spec#/

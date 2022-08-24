---
title: "Your First Transaction using the SDK"
slug: "your-first-transaction-sdk"
sidebar_position: 0
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Transaction using the SDK

This tutorial introduces the Aptos SDKs and how to generate, submit, and verify transactions submitted to the Aptos Blockchain.

## Step 1: Pick an SDK

* [Official Aptos Python SDK][python-sdk]
* Official Aptos Typescript SDK — *TBA*
* Official Aptos Rust SDK — *TBA*

## Step 2: Run the Example

Each SDK provides an examples directory. For the Python SDK, the examples directory is located here: `https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk/examples`. 

This tutorial covers the [`transfer-coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/transfer-coin.py) example.

<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

Run the `transfer-coin` example in the Python SDK directory, i.e., `/path/to/aptos-core/ecosystem/python/sdk`. For example:

```python
cd /path/to/aptos-core/ecosystem/python/sdk
python3 -m examples.transfer-coin
```

  </TabItem>
  <TabItem value="rust" label="Rust">

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

## Step 3: Understand the Output

An output very similar to the following will appear after executing the above command:

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

The above output demonstrates that the `transfer-coin` example executes the following steps:

* Initializing the REST and faucet clients.
* The creation of two accounts: Alice and Bob.
  * The funding and creation of Alice's account from a faucet.
  * The creation of Bob's account from a faucet.
* The transferring of 1000 coins from Alice to Bob.
* The 4 coins of gas paid for by Alice to make that tansfer.
* Another transfer of 1000 coins from Alice to Bob.
* The additional 4 coins of gas paid for by Alice to make that transfer.

Next, see below a walk-through of the Python SDK functions that are used to accomplish the above steps.

## Step 4: The SDK in Depth

The `transfer-coin` example code uses helper functions to interact with the [REST API][rest_spec]. This section reviews each of the calls and gives insights into functionality.

:::tip See the example full listing
See the [`transfer-coin`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/transfer-coin.py) for the complete code as you follow the below steps.
:::

### Step 4.1: Initializing the Clients

In the first step, the `transfer-coin` example initializes both the REST and faucet clients. 

- The REST client interacts with the REST API, and
- The faucet client interacts with the devnet Faucet service for creating and funding accounts.

<Tabs groupId="sdk-examples">
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

In progress.
  </TabItem>
  <TabItem value="typescript" label="Typescript">

In progress.
  </TabItem>
</Tabs>

:::tip

By default the URLs for both the services point to Aptos devnet services. However, they can be configured with the following environment variables: 
  - `APTOS_NODE_URL` and
  - `APTOS_FAUCET_URL`.
:::

### Step 4.2: Creating local accounts

The next step is to create two accounts from the locally. [Accounts][account_basics] represent both on-chain and off-chain state. Off-chain state consists of an address and the public, private key pair used to authenticate ownership. This step demonstrates how to generate that off-chain state.

<Tabs groupId="sdk-examples">
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

In Aptos, each account must have an on-chain representation in order to support receive tokens and coins as well as interacting in other dApps. An account represents a medium for storing assets, hence it must be explicitly created. This example leverages the Faucet to create and fund Alice's account and to only create Bob's account:

<Tabs groupId="sdk-examples">
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

In this step, the Python SDK translates a single call into the process of querying a resource and reading a field from that resource.

<Tabs groupId="sdk-examples">
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

Like the previous step, this is another helper step that constructs a transaction which transfers the coins from Alice to Bob. For correctly generated transactions, the API will return a transaction hash that can be used in the subsequent step to check on the transaction status. The Aptos blockchain does perform a handful of validation checks on submission and if any of those fail, the user will instead be given an error. These validations include the transaction signature, unused sequence number, and submitting the transaction to the appropriate chain.

<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/transfer-coin.py section_5
```

Behind the scenes the Python SDK generates, signs, and submits a transaction:
```python
:!: static/sdks/python/aptos_sdk/client.py bcs_transfer
```

where:

1. The function `transfer` is internally a `EntryFunction`, i.e., an entry function in Move that is directly callable.
2. The Move function is stored on the coin module: `0x1::coin`.
3. Because the `coin` module can be used by other coins, the transfer must explicitly use a `TypeTag` to define which coin to transfer.
4. The transaction arguments must be placed into `TransactionArgument`s with type specifiers (`Serializer.{type}`), that will serialize the value into the appropriate type at transaction generation time.

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

<Tabs groupId="sdk-examples">
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

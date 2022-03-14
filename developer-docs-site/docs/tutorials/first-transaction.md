---
title: "Your first transaction"
slug: "your-first-transaction"
sidebar_position: 1
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your first transaction

This tutorial details how to generate, submit, and verify transactions submitted to the Aptos Blockchain. The steps to doing so:

* Create a representation of an account
* Prepare a wrapper around the REST interfaces
* Prepare a wrapper around the Faucet interface
* Combine them into an application, execute and verify

The following tutorial contains example code that can be downloaded from our github below:

<Tabs>
  <TabItem value="python" label="Python" default>

For this tutorial, will be focusing on `first_transaction.py`.

You can find the python project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/python)

  </TabItem>
  <TabItem value="rust" label="Rust" default>

For this tutorial, will be focusing on `first_transaction.rs`.

You can find the rust project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/rust)

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

For this tutorial, will be focusing on `first_transaction.ts`.

You can find the typescript project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript)

  </TabItem>
</Tabs>

## Step 1) Create a representation of an account

Each Aptos account has a unique account address.  The owner of that account holds the public, private key-pair that maps to the Aptos account address and, in turn, the authentication key stored in that account.  See more in [account basics][account_basics]. The following snippets demonstrate what's described in that section.

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_transaction.py section_1
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_transaction/src/lib.rs section_1
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_transaction.ts section_1
```

  </TabItem>
</Tabs>

## Step 2) REST interface

Aptos exposes a [REST interface][rest_spec] for interacting with the blockchain. While the data from the REST interface can be read directly, the following snippets of code demonstrate a more ergonomic approach. This next set of code snippets demonstrates how to use the REST interface to retrieve ledger data from the FullNode including account and account resource data. It also demonstrates how to use the REST interface for constructing a signed transactions represented by JSON formatting.

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_transaction.py section_2
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_transaction/src/lib.rs section_2
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_transaction.ts section_2
```

  </TabItem>
</Tabs>

### Step 2.1) Reading an account

The following are wrappers for querying account data.

<Tabs>
  <TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_transaction.py section_3
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_transaction/src/lib.rs section_3
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_transaction.ts section_3
```

  </TabItem>
</Tabs>

### Step 2.2) Submitting a transaction

The following demonstrate the core functionality for constructing, signing, and waiting on a transaction.
<Tabs>
<TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_transaction.py section_4
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_transaction/src/lib.rs section_4
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_transaction.ts section_4
```

  </TabItem>
</Tabs>

### Step 2.3) Application specific logic

The following demonstrate how to read data from the blockchain and how to write to it, e.g., submit a specific transaction.

<Tabs>
<TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_transaction.py section_5
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_transaction/src/lib.rs section_5
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_transaction.ts section_5
```

  </TabItem>
</Tabs>

## Step 3) Faucet interface

A blockchain faucet provides an account some amount of tokens that can be used for paying gas fees or transferring of tokens betwen users. The Aptos faucet additionally can create an account if one does not exist yet. The Aptos faucet interface requires a public key represented in a hex-encoded string.

<Tabs>
<TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_transaction.py section_6
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_transaction/src/lib.rs section_6
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_transaction.ts section_6
```

  </TabItem>
</Tabs>

## Step 4) Execute the application and verify

<Tabs>
<TabItem value="python" label="Python" default>

```python
:!: static/examples/python/first_transaction.py section_7
```

  </TabItem>
  <TabItem value="rust" label="Rust" default>

```rust
:!: static/examples/rust/first_transaction/src/main.rs section_7
```

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/first_transaction.ts section_7
```

  </TabItem>
</Tabs>

The output after executing:
```
=== Addresses ===
Alice: e26d69b8d3ff12874358da6a4082a2ac
Bob: c8585f009c8a90f22c6b603f28b9ed8c

=== Initial Balances ===
Alice: 1000000000
Bob: 0

=== Final Balances ===
Alice: 999998957
Bob: 1000
```

The outcome shows that Bob received 1000 coins from Alice. Alice paid 43 coins for gas.

The data can be verified by visiting either a REST interface or the explorer:
* Alice's account via the [REST interface][alice_account_rest]
* Bob's account via the [explorer][bob_account_explorer]

:::info
The devnet gets cleared out from time to time, so the above links may not work.<br/> Try the tutorial yourself and check their accounts in the explorer after!

[account_basics]: /basics/basics-accounts
[alice_account_rest]: https://dev.fullnode.aptoslabs.com/accounts/e26d69b8d3ff12874358da6a4082a2ac/resources
[bob_account_explorer]: https://aptos-explorer.netlify.app/account/c8585f009c8a90f22c6b603f28b9ed8c
[rest_spec]: https://dev.fullnode.aptoslabs.com/spec.html

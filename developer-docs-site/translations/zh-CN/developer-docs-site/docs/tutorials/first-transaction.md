---
title: "Your First Transaction"
slug: "your-first-transaction"
sidebar_position: 1
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Transaction

This tutorial describes, in the following step-by-step approach, how to generate, submit, and verify transactions submitted to the Aptos Blockchain:

1. Create a representation of an account.

  Each Aptos account has a unique account address. The owner of that account holds the public, private key-pair that maps to the Aptos account address and, in turn, the authentication key stored in that account.

  :::note See more about Aptos accounts in [Accounts][account_basics]. :::

2. Prepare a wrapper around the REST interfaces.

  Aptos provides a [REST API][rest_spec] for interacting with the blockchain. This steps prepares wrappers around this API, for retrieving account information, and for constructing a transaction, signing it and submitting the transaction.

3. Prepare a wrapper around the Faucet interface.

  Using the Faucet interface at the Aptos devnet, this tutorial code automatically creates an account with the account address `0x1` and funds the account.

4. Combine the above wrappers into an application, execute and verify.

## Before you start

Make sure you follow the below steps first so you can run the tutorial.

1. Clone the Aptos repo.

      ```
      git clone https://github.com/aptos-labs/aptos-core.git

      ```

2. `cd` into `aptos-core` directory.

    ```
    cd aptos-core
    ```
3. Checkout the devnet branch using `git checkout --track origin/devnet`.

4. Run the `scripts/dev_setup.sh` Bash script as shown below. This will prepare your developer environment.

    ```
    ./scripts/dev_setup.sh
    ```

5. Update your current shell environment.

    ```
    source ~/.cargo/env
    ```

With your development environment ready, now you are ready to run this tutorial.

## GitHub source

Follow the below links to access the source code for the tutorial:

<Tabs>
  <TabItem value="python" label="Python" default>

See the `first_transaction.py` code in the [Python version](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/python) of the tutorial.

  </TabItem>
  <TabItem value="rust" label="Rust" default>

See the `first_transaction.rs` code in the [Rust project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/rust) of the tutorial.

  </TabItem>
  <TabItem value="typescript" label="Typescript" default>

See the `first_transaction.ts` code in the [Typescript project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript) of the tutorial.

  </TabItem>
</Tabs>

## Step 1: Create a representation of an account

This steps creates the representation of an account. See also [Aptos accounts][account_basics] and [Creating a Signed Transaction](/docs/guides/sign-a-transaction.md).

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

## Step 2: REST interface

While the data from the REST interface can be read directly, the following code examples demonstrate a more ergonomic approach, while still using the REST interface, for:

- Retrieving the ledger data from the FullNode, including account and account resource data.
- Constructing signed transactions, represented by JSON format.

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

### Step 2.1: Reading an account

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

### Step 2.2: Submitting a transaction

The following demonstrates the core functionality for constructing, signing, and waiting on a transaction.

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

### Step 2.3: Application-specific logic

The following demonstrates how to read data from the blockchain and how to submit a specific transaction.

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

## Step 3: Faucet interface

Aptos Blockchain faucets issue test tokens to accounts. These test tokens can be used for testing, e.g., paying gas fees or transferring tokens between users. The Aptos Faucet can also create accounts if they do not exist. The Aptos Faucet interface requires a public key represented in a hex-encoded string.

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

## Step 4: Run the application

Finally, we can run the application and verify the output.

<Tabs>
<TabItem value="python" label="Python" default>
For Python3:

1. Make sure you followed the prerequisites described in [Before you start](#before-you-start). 
2. `cd` into `aptos-core/developer-docs-site/static/examples/python` directory.
3. Install the required libraries: `pip3 install -r requirements.txt`.
4. Run the example: `python3 first_transaction.py`.

</TabItem>
<TabItem value="rust" label="Rust">
For Rust:

1. Make sure you followed the prerequisites described in [Before you start](#before-you-start). 
2. `cd` into `aptos-core/developer-docs-site/static/examples/rust` directory.
3. Execute the example: `cargo run --bin first-transaction` (make sure you use `first-transaction` and not `first_transaction`).

</TabItem>
<TabItem value="typescript" label="Typescript">
For Typescript:

1. Make sure you followed the prerequisites described in [Before you start](#before-you-start). 
2. `cd` into `aptos-core/developer-docs-site/static/examples/typescript` directory.
3. Install the required libraries: `yarn install`.
4. Execute the example: `yarn first_transaction`.

</TabItem>
</Tabs>

### Output

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

The output shows that Bob received 1000 coins from Alice. Alice paid 43 coins for gas.

### Verify

The data can be verified by visiting either a REST interface or the explorer:
* Alice's account via the [Aptos REST interface][alice_account_rest].
* Bob's account via the [Aptos Explorer][bob_account_explorer].

:::note The Aptos devnet is reset from time to time, so the above links may not work. Try the tutorial yourself and check the accounts in the [Aptos Explorer][bob_account_explorer] then.

[account_basics]: /basics/basics-accounts

[account_basics]: /basics/basics-accounts
[alice_account_rest]: https://fullnode.devnet.aptoslabs.com/accounts/e26d69b8d3ff12874358da6a4082a2ac/resources
[bob_account_explorer]: https://aptos-explorer.netlify.app/account/c8585f009c8a90f22c6b603f28b9ed8c
[rest_spec]: https://fullnode.devnet.aptoslabs.com/spec.html

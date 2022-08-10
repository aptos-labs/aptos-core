---
title: "Guide for Move Transactional Testing"
slug: "guide-move-transactional-testing"
---

# Guide for Move Transactional Testing

:::caution Exploratory feature

The Move transactional testing feature described in this document is in exploratory phase. Support for it will depend on its usage and adoption by the Aptos community. 
:::



If you are a smart contract developer using the Move language, then you can use the Move transactional tests to write and run end-to-end tests. 

This tutorial walks you through the steps for writing and running end-to-end Move transactional tests using the [Aptos CLI](/cli-tools/aptos-cli-tool/index.md). 

Compared to the Move unit tests, which are useful for verifying the intra-module code correctness, the Move transactional tests enable you to test a broader spectrum of use cases, such as publishing the Move modules and the inter-module interactions. 

## Overview

See this `aptos_test_harness` [GitHub folder](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/aptos-transactional-test-harness/tests/aptos_test_harness) for how Move transactional tests look like. 

A Move transactional test suite consists of two types of files:

- The Move files, with`*.move` extension. These Move files contain the transactional tests. Transactional tests are Rust [clap](https://docs.rs/clap/latest/clap/) (Command Line Argument Parser ) commands. They are written as comments in the Move files. To distinguish from the normal Move comments, these transactional test commands are prefixed with a special comment string indicator `//#` .
- The baseline files, with `*.exp`extension. These baseline files will be created empty when you run the `aptos` CLI for the first time. If you run the test with `UB=1` option the first time, the baseline files will be populated with the test output.

## Quickstart

Before you get into the details, you can follow the below steps to run a sample Move transactional test and see the results. 

### Step 1: Install Aptos CLI

Make sure you have installed `aptos` , the Aptos CLI tool. See [Aptos CLI](/cli-tools/aptos-cli-tool/index.md) for how to install and use the `aptos` CLI tool. 

### Step 2: Run the Move transactional test suite

- Clone or download the `aptos-core` [GitHub repo](https://github.com/aptos-labs/aptos-core.git).
- The Move transactional test suite is located in `aptos-core/aptos-move/aptos-transactional-test-harness/test` .
- Run the `aptos` CLI command with the option `move transactional-test`.
:::tip Use UB=1
When you run the `aptos` CLI command make sure to include the `UB=1` **only during the first time or if you have updated the tests**, as shown below. 
:::

```bash
UB=1 aptos move transactional-test --root-path aptos-core/aptos-move/aptos-transactional-test-harness/test
```
- The `--root-path` specifies where all the tests are located. 
- The test runner walks down the directory hierarchies and finds the tests, specified as comments with the special prefix `//#`, in the files whose names end with `.move` or `.mvir`. 
- The Move transactional test runner runs the tests and compares the output of the tests with the baseline files. Baseline files hold the contents of the results that were generated during the first run. 

## Examples

This section presents examples showing how to write and run various Move transactional test commands.

### Create accounts

```rust
//# create_account —name Alice [--initial-coins 10000]
```

The `create_account` command generates a deterministic private key,  public key pair and creates a named account address (Alice in the above example). 

:::tip Default value
Initial coins can be specified, otherwise, a default value of `10000` is used. 
:::

### Publish modules

```rust
//# publish [--gas-budget 100]
module Alice::first_module {
	public entry fun foo() {
		return
	}
}
```

The `publish` command publishes a Move module to a designated account (**Alice** in the above example). Optionally, the number of gas units allowed for publishing the transaction can be specified via `--gas-budget`. 

:::tip Default value
The default value is the maximum coins available at the sender account.
:::

### Run module script functions

```rust
//# run --signers Alice [--args x"68656C6C6F20776F726C64"] [--type-args "0x1::aptos_coin::AptosCoin"] [--expiration 1658432810] [--sequence-number 1] [--gas-price 1] [--show-events] -- Alice::first_module::function_name
```

The `run` command runs a module script function by sending a transaction. 

In the above example:

- `--signers` specify who signs and sends the transaction.
- `Alice::first_module::function_name` is the fully qualified Move module function name.
- `--args` specify the arguments to pass to the script function.
- `--type-args` specify the type arguments if the script function is a generic function.
- `--expiration` transaction expiration time.
- `--sequence-number` account sequence number.
- `--gas-price` gas unit price.
- `--show-events` print out the transaction events if specified.

### Execute scripts

```rust
//# run --script --signers Alice [--args x"68656C6C6F20776F726C64"] [--type-args "0x1::aptos_coin::AptosCoin"] [--expiration 1658432810] [--sequence-number 1] [--gas-price 1]
script {
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;

    fun main(sender: &signer, receiver: address, amount: u64) {
        coin::transfer<AptosCoin>(sender, receiver, amount);
    }
}
```

The `run` command can also be used with the `--script` option to execute a script.

### View resources

```rust
//# view_resource --address Alice --type 0x1::coin::CoinStore<0x1::test_coin::TestCoin> [--field coin.value]
```

The `view_resource` prints out the resources contained at an address.

- `--address` of the account whose resources should be printed.
- `--type` the type of resource to be printed.
- `--field` prints out only the specified field.

### View tables

```rust
//# view_table --table_handle 5713946181763753045826830927579154558 --key_type 0x1::string::String --key_value x"68656C6C6F20776F726C64" --value_type 0x1::token::Collection 
```

The `view_table` prints out an item in a table by the item’s key. For example, in the below Move code:

```rust
struct Collections has key {
	collections: Table<string::String, Collection>,
}
```

- In storage, the table `collections` has a handle stored in resource `Collections`.
- To query a table item by key, we need to know the table handle information `--table_handle 5713946181763753045826830927579154558`.
- The`table_handle` is handle of the table to be queried. It can be looked up by the command `view_resource`.
- The `--key_type` is the type info for the key. To get the value for a key, use the `view_table` option with `--key_type` and `--key_value`.
- The `--key_value` is the key that is used to retrieve the value.
- The `key_type` is used to deserialize the `key_value` to obtain a raw key, which can be used to query storage.
- The `--value_type` the type information for deserializing the table value. The storage return value also must be deserialized to be useful. Therefore, `--value_type` is also needed.

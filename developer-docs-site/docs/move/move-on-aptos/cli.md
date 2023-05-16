---
title: "Aptos Move CLI"
---

# Use the Aptos Move CLI

The `aptos` tool is a command line interface (CLI) for developing on the Aptos blockchain, debugging, and for node operations. This document describes how to use the `aptos` CLI tool. To download or build the CLI, follow [Install Aptos CLI](../../tools/install-cli/index.md).

## Compiling Move

The `aptos` CLI can be used to compile a Move package locally.
The below example uses the `HelloBlockchain` in [move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples).

The named addresses can be either an account address, or a profile name.

```bash
$ aptos move compile --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses hello_blockchain=superuser
```

The above command will generate the below terminal output:
```bash
{
  "Result": [
    "742854F7DCA56EA6309B51E8CEBB830B12623F9C9D76C72C3242E4CAD353DEDC::Message"
  ]
}
```

## Compiling and unit testing Move

The `aptos` CLI can also be used to compile and run unit tests locally.
In this example, we'll use the `HelloBlockchain` in [move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples).

```bash
$ aptos move test --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses hello_blockchain=superuser
```
The above command will generate the following terminal output:
```bash
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING Examples
Running Move unit tests
[ PASS    ] 0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc::MessageTests::sender_can_set_message
[ PASS    ] 0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc::Message::sender_can_set_message
Test result: OK. Total tests: 2; passed: 2; failed: 0
{
  "Result": "Success"
}
```
## Generating test coverage details for Move
The `aptos` CLI can be used to analyze and improve the testing of your Move modules. To use this feature:
1. In your `aptos-core` source checkout, navigate to the `aptos-move/framework/move-stdlib` directory.
2. Execute the command:
   ```bash
   $ aptos move test --coverage
   ```
3. Receive results in standard output containing the result for each test case followed by a basic coverage summary resembling:
   ```bash
   BUILDING MoveStdlib
Running Move unit tests
[ PASS    ] 0x1::vector_tests::append_empties_is_empty
[ PASS    ] 0x1::option_tests::borrow_mut_none
[ PASS    ] 0x1::fixed_point32_tests::ceil_can_round_up_correctly
[ PASS    ] 0x1::features::test_change_feature_txn
[ PASS    ] 0x1::bcs_tests::bcs_bool
[ PASS    ] 0x1::bit_vector_tests::empty_bitvector
[ PASS    ] 0x1::option_tests::borrow_mut_some
Test result: OK. Total tests: 149; passed: 149; failed: 0
+-------------------------+
| Move Coverage Summary   |
+-------------------------+
Module 0000000000000000000000000000000000000000000000000000000000000001::bcs
>>> % Module coverage: NaN
Module 0000000000000000000000000000000000000000000000000000000000000001::fixed_point32
>>> % Module coverage: 100.00
Module 0000000000000000000000000000000000000000000000000000000000000001::hash
>>> % Module coverage: NaN
Module 0000000000000000000000000000000000000000000000000000000000000001::vector
>>> % Module coverage: 92.19
Module 0000000000000000000000000000000000000000000000000000000000000001::error
>>> % Module coverage: 0.00
Module 0000000000000000000000000000000000000000000000000000000000000001::acl
>>> % Module coverage: 0.00
Module 0000000000000000000000000000000000000000000000000000000000000001::bit_vector
>>> % Module coverage: 97.32
Module 0000000000000000000000000000000000000000000000000000000000000001::signer
>>> % Module coverage: 100.00
Module 0000000000000000000000000000000000000000000000000000000000000001::features
>>> % Module coverage: 69.41
Module 0000000000000000000000000000000000000000000000000000000000000001::option
>>> % Module coverage: 100.00
Module 0000000000000000000000000000000000000000000000000000000000000001::string
>>> % Module coverage: 81.82
+-------------------------+
| % Move Coverage: 83.50  |
+-------------------------+
Please use `aptos move coverage -h` for more detailed test coverage of this package
{
  "Result": "Success"
}
   ```

4. Optionally, narrow down your test runs and results to a specific package name with the `--filter` option, like so:
   ```bash
   $ aptos move test --coverage --filter vector
   ```

   With results like:
   ```
   BUILDING MoveStdlib
   Running Move unit tests
   [ PASS    ] 0x1::bit_vector_tests::empty_bitvector
   [ PASS    ] 0x1::vector_tests::append_empties_is_empty
   [ PASS    ] 0x1::bit_vector_tests::index_bit_out_of_bounds
   [ PASS    ] 0x1::vector_tests::append_respects_order_empty_lhs
   ```
5. Run the `aptos move coverage` command to obtain more detailed coverage information.
6. Optionally, isolate the results to a module by passing its name to the `--module` option, for example:
   ```bash
   $ aptos move coverage source --module signer
   ```

   With results:
   ```
   module std::signer {
       // Borrows the address of the signer
       // Conceptually, you can think of the `signer` as being a struct wrapper arround an
       // address
       // ```
       // struct signer has drop { addr: address }
       // ```
       // `borrow_address` borrows this inner field
       native public fun borrow_address(s: &signer): &address;

       // Copies the address of the signer
       public fun address_of(s: &signer): address {
           *borrow_address(s)
       }

    /// Return true only if `s` is a transaction signer. This is a spec function only available in spec.
    spec native fun is_txn_signer(s: signer): bool;

    /// Return true only if `a` is a transaction signer address. This is a spec function only available in spec.
    spec native fun is_txn_signer_addr(a: address): bool;
}
{
  "Result": "Success"
}
   ```
6. Find failures and iteratively improve your testing and running these commands to eliminate gaps in your testing coverage.

## Proving Move

The `aptos` CLI can be used to run [Move Prover](../../move/prover/index.md), which is a formal verification tool for the Move language. The below example proves the `hello_prover` package in [move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples).
```bash
aptos move prove --package-dir aptos-move/move-examples/hello_prover/
```
The above command will generate the following terminal output:
```bash
SUCCESS proving 1 modules from package `hello_prover` in 1.649s
{
  "Result": "Success"
}
```

Move Prover may fail with the following terminal output if the dependencies are not installed and set up properly:
```bash
FAILURE proving 1 modules from package `hello_prover` in 0.067s
{
  "Error": "Move Prover failed: No boogie executable set.  Please set BOOGIE_EXE"
}
```
In this case, see [Install the dependencies of Move Prover](../../tools/install-cli/index.md#step-3-optional-install-the-dependencies-of-move-prover).

## Profiling gas usage

This *experimental* feature lets you [profile gas usage](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/aptos-gas-profiling) in the Aptos virtual machine locally rather than [simulating transactions](../../concepts/gas-txn-fee.md#estimating-the-gas-units-via-simulation) at the [fullnode](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/simulate_transaction). You may also use it to visualize gas usage in the form of a flame graph.

Run the gas profiler by appending the `--profile-gas` option to the Aptos CLI `move publish`, `move run` or `move run-script` command, for example:
```bash
aptos move publish --profile-gas
```

And receive output resembling:
```bash
Compiling, may take a little while to download git dependencies...
BUILDING empty_fun
package size 427 bytes
Simulating transaction locally with the gas profiler...
This is still experimental so results may be inaccurate.
Execution & IO Gas flamegraph saved to gas-profiling/txn-69e19ee4-0x1-code-publish_package_txn.exec_io.svg
Storage fee flamegraph saved to gas-profiling/txn-69e19ee4-0x1-code-publish_package_txn.storage.svg
{
  "Result": {
    "transaction_hash": "0x69e19ee4cc89cb1f84ee21a46e6b281bd8696115aa332275eca38c4857818dfe",
    "gas_used": 1007,
    "gas_unit_price": 100,
    "sender": "dbcbe741d003a7369d87ec8717afb5df425977106497052f96f4e236372f7dd5",
    "success": true,
    "version": 473269362,
    "vm_status": "status EXECUTED of type Execution"
  }
}
```

Find the flame graphs in the newly created `gas-profiling/` directory. To interact with a graph, open the file in a web browser.

Note these limitations of the experimental gas profiling feature:

  * It may produce results that are different from the simulation.
  * The graphs may contain errors, and the numbers may not add up to the total gas cost as shown in the transaction output.

## Debugging and printing stack trace

In this example, we will use `DebugDemo` in [debug-move-example](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos/debug-move-example).

Now, you can use `debug::print` and `debug::print_stack_trace` in your [DebugDemo Move file](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos/debug-move-example/sources/DebugDemo.move).

You can run the following command:
```bash
$ aptos move test --package-dir crates/aptos/debug-move-example
```

The command will generate the following output:
```bash
Running Move unit tests
[debug] 0000000000000000000000000000000000000000000000000000000000000001
Call Stack:
    [0] 0000000000000000000000000000000000000000000000000000000000000001::Message::sender_can_set_message

        Code:
            [4] CallGeneric(0)
            [5] MoveLoc(0)
            [6] LdConst(0)
          > [7] Call(1)
            [8] Ret

        Locals:
            [0] -
            [1] 0000000000000000000000000000000000000000000000000000000000000001


Operand Stack:
```


## Publishing a Move package with a named address

In this example, we'll use the `HelloBlockchain` in [move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples).

Publish the package with your account address set for `HelloBlockchain`.

Here, you need to change 8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71 to your account address.
```bash
$ aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71
```

:::tip
As an open source project, the source code as well as compiled code published to the Aptos blockchain is inherently open by default. This means code you upload may be downloaded from on-chain data. Even without source access, it is possible to regenerate Move source from Move bytecode. To disable source access, publish with the `--included-artifacts none` argument, like so:

```
aptos move publish --included-artifacts none
```
:::

You can additionally use named profiles for the addresses.  The first placeholder is `default`
```bash
$ aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=default
```

:::tip
When publishing Move modules, if multiple modules are in one package, then all the modules in this package must have the same account. If they have different accounts, then the publishing will fail at the transaction level.
:::

## Running a Move function

Now that you've published the function above, you can run it.

Arguments must be given a type with a colon to separate it.  In this example, we want the input to be
parsed as a string, so we put `string:Hello!`.

```bash
$ aptos move run --function-id 0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb::message::set_message --args string:hello!
{
  "Result": {
    "changes": [
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "authentication_key": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
          "self_address": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
          "sequence_number": "3"
        },
        "event": "write_resource",
        "resource": "0x1::account::Account"
      },
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "coin": {
            "value": "9777"
          },
          "deposit_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
                "creation_num": "1"
              }
            }
          },
          "withdraw_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
                "creation_num": "2"
              }
            }
          }
        },
        "event": "write_resource",
        "resource": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"
      },
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "counter": "4"
        },
        "event": "write_resource",
        "resource": "0x1::guid::Generator"
      },
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "message": "hello!",
          "message_change_events": {
            "counter": "0",
            "guid": {
              "id": {
                "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
                "creation_num": "3"
              }
            }
          }
        },
        "event": "write_resource",
        "resource": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb::Message::MessageHolder"
      }
    ],
    "gas_used": 41,
    "success": true,
    "version": 3488,
    "vm_status": "Executed successfully"
  }
}
```

Additionally, profiles can replace addresses in the function id.
```bash
$ aptos move run --function-id default::message::set_message --args string:hello!
{
  "Result": {
    "changes": [
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "authentication_key": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
          "self_address": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
          "sequence_number": "3"
        },
        "event": "write_resource",
        "resource": "0x1::account::Account"
      },
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "coin": {
            "value": "9777"
          },
          "deposit_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
                "creation_num": "1"
              }
            }
          },
          "withdraw_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
                "creation_num": "2"
              }
            }
          }
        },
        "event": "write_resource",
        "resource": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"
      },
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "counter": "4"
        },
        "event": "write_resource",
        "resource": "0x1::guid::Generator"
      },
      {
        "address": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "data": {
          "message": "hello!",
          "message_change_events": {
            "counter": "0",
            "guid": {
              "id": {
                "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
                "creation_num": "3"
              }
            }
          }
        },
        "event": "write_resource",
        "resource": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb::Message::MessageHolder"
      }
    ],
    "gas_used": 41,
    "success": true,
    "version": 3488,
    "vm_status": "Executed successfully"
  }
}
```

## Arguments in JSON

### Background

This section references the [`CliArgs` example package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args), which contains the following manifest:


```toml title="Move.toml"
:!: static/move-examples/cli_args/Move.toml manifest
```

Here, the package is deployed under the named address `test_account`.

### Entry functions

The only module in the package, `cli_args.move`, defines a simple `Holder` resource with fields of various data types:

```rust title="Holder in cli_args.move"
:!: static/move-examples/cli_args/sources/cli_args.move resource
```

A public entry function with multi-nested vectors can be used to set the fields:

```rust title="Setter function in cli_args.move"
:!: static/move-examples/cli_args/sources/cli_args.move setter
```

After the package has been published, `aptos move run` can be used to call `set_vals()`:

```zsh title="Running function with nested vector arguments from CLI"
aptos move run \
    --function-id <test_account>::cli_args::set_vals \
    --private-key-file <test_account.key> \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args \
        u8:123 \
        "bool:[false, true, false, false]" \
        'address:[["0xace", "0xbee"], ["0xcad"], []]'
```

:::tip
To pass vectors (including nested vectors) as arguments from the command line, use JSON syntax escaped with quotes!
:::

The function ID, type arguments, and arguments can alternatively be specified in a JSON file:

import CodeBlock from '@theme/CodeBlock';
import entry_json_file from '!!raw-loader!../../../../aptos-move/move-examples/cli_args/entry_function_arguments.json';

<CodeBlock language="json" title="entry_function_arguments.json">{entry_json_file}</CodeBlock>

Here, the call to `aptos move run` looks like:

```zsh
aptos move run \
    --private-key-file <test_account.key> \
    --json-file entry_function_arguments.json
```

:::tip
If you are trying to run the example yourself don't forget to substitute `<test_account>` with an appropriate address in the JSON file from the [`CliArgs` example package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args)!
:::

### View functions

Once the values in a `Holder` have been set, the `reveal()` view function can be used to check the first three fields, and to compare type arguments against the last two fields:

```rust title="View function"
:!: static/move-examples/cli_args/sources/cli_args.move view
```

This view function can be called with arguments specified either from the CLI or from a JSON file:

```zsh title="Arguments via CLI"
aptos move view \
    --function-id <test_account>::cli_args::reveal \
    --type-args \
        0x1::account::Account \
        0x1::account::Account \
    --args address:<test_account>
```

```zsh title="Arguments via JSON file"
aptos move view --json-file view_function_arguments.json
```

import view_json_file from '!!raw-loader!../../../../aptos-move/move-examples/cli_args/view_function_arguments.json';

<CodeBlock language="json" title="view_function_arguments.json">{view_json_file}</CodeBlock>

```zsh title="Output"
{
  "Result": [
    123,
    [
      false,
      true,
      false,
      false
    ],
    [
      [
        "0xace",
        "0xbee"
      ],
      [
        "0xcad"
      ],
      []
    ],
    true,
    false
  ]
}
```

### Script functions

The package also contains a script, `set_vals.move`, which is a wrapper for the setter function:

```rust title="script"
:!: static/move-examples/cli_args/scripts/set_vals.move script
```

Here, `aptos move run-script` is run from inside the [`cli_args` package directory](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args):

:::tip
Before trying out the below examples, compile the package with the correct named address via:

```zsh
aptos move compile --named-addresses test_account=<test_account>
```
:::

```zsh title="Arguments via CLI"
aptos move run-script \
    --compiled-script-path build/CliArgs/bytecode_scripts/set_vals.mv \
    --private-key-file <test_account.key> \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args \
        u8:123 \
        "u8:[122, 123, 124, 125]" \
        address:"0xace"
```

```zsh title="Arguments via JSON file"
aptos move run-script \
    --compiled-script-path build/CliArgs/bytecode_scripts/set_vals.mv \
    --private-key-file <test_account.key> \
    --json-file script_function_arguments.json
```

import script_json_file from '!!raw-loader!../../../../aptos-move/move-examples/cli_args/script_function_arguments.json';

<CodeBlock language="json" title="script_function_arguments.json">{script_json_file}</CodeBlock>

Both such script function invocations result in the following `reveal()` view function output:

```zsh title="View function call"
aptos move view \
    --function-id <test_account>::cli_args::reveal \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args address:<test_account>
```

```json title="View function output"
{
  "Result": [
    123,
    [
      false,
      false,
      true,
      true
    ],
    [
      [
        "0xace"
      ]
    ],
    true,
    true
  ]
}
```

:::note
As of the time of this writing, the `aptos` CLI only supports script function arguments for vectors of type `u8`, and only up to a vector depth of 1. Hence `vector<address>` and `vector<vector<u8>>` are invalid script function argument types.
:::


## Multisig governance

### Background

This section builds upon the [Arguments in JSON](#arguments-in-json) section, and likewise references the [`CliArgs` example package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args).

:::tip
Set your working directory to [`aptos-move/move-examples/cli_args`](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args) to follow along:

```bash
cd <aptos-core-parent-directory>/aptos-core/aptos-move/move-examples/cli_args
```
:::


For this example, Ace and Bee will conduct governance operations from a 2-of-2 multisig account.

### Account creation

Start by mining a vanity address for both signatories:

```bash title=Command
aptos key generate \
    --vanity-prefix 0xace \
    --output-file ace.key
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "PublicKey Path": "ace.key.pub",
    "Account Address:": "{
  "Result": {
    "PublicKey Path": "bee.key.pub",
    "PrivateKey Path": "bee.key",
    "Account Address:": "0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4"
  }
}",
    "PrivateKey Path": "ace.key"
  }
}
```

</details>

```bash title=Command
aptos key generate \
    --vanity-prefix 0xbee \
    --output-file bee.key
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "PublicKey Path": "bee.key.pub",
    "PrivateKey Path": "bee.key",
    "Account Address:": "0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4"
  }
}
```

</details>

:::tip
The exact account address should vary for each run, though the vanity prefix should not.
:::

Store Ace and Bee's addresses in shell variables so you can call them inline later on:

```bash
# Your exact addresses should vary
ace_addr=0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4
bee_addr=0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4
```

Now fund Ace's and Bee's accounts using the faucet:

```bash title=Command
aptos account fund-with-faucet --account $ace_addr
```

<details><summary>Output</summary>

```bash
{
  "Result": "Added 100000000 Octas to account acee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4"
}
```

</details>

```bash title=Command
aptos account fund-with-faucet --account $bee_addr
```

<details><summary>Output</summary>

```bash
{
  "Result": "Added 100000000 Octas to account bee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4"
}
```

</details>

Ace can now create a multisig account:

```bash title=Command
aptos multisig create \
    --additional-owners $bee_addr \
    --num-signatures-required 2 \
    --private-key-file ace.key
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "multisig_address": "2dc9b2fdba8ace3b9f96e5d3bb7ea04d39b1640020c8697eb0f1f4b33cad0d77",
    "transaction_hash": "0x9b566b9357c1cda768948f6aaf951c6d5e5c3e1749c2fb3147b5eed371e962ee",
    "gas_used": 1524,
    "gas_unit_price": 100,
    "sender": "acee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1684644958857234,
    "version": 525304840,
    "vm_status": "Executed successfully"
  }
}
```

</details>

Store the multisig address in a shell variable:

```bash
# Your address should vary
multisig_addr=2dc9b2fdba8ace3b9f96e5d3bb7ea04d39b1640020c8697eb0f1f4b33cad0d77
```

### Inspect the multisig

Use the assorted [`multisig_account.move` view functions](https://github.com/aptos-labs/aptos-core/blob/9fa0102c3e474d99ea35a0a85c6893604be41611/aptos-move/framework/aptos-framework/sources/multisig_account.move#L237) to inspect the multisig:

```bash title="Number of signatures required"
aptos move view \
    --function-id 0x1::multisig_account::num_signatures_required \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    "2"
  ]
}
```

</details>

```bash title="Owners"
aptos move view \
    --function-id 0x1::multisig_account::owners \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    [
      "0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4",
      "0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4"
    ]
  ]
}
```

</details>

```bash title="Last resolved sequence number"
aptos move view \
    --function-id 0x1::multisig_account::last_resolved_sequence_number \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    "0"
  ]
}
```

</details>

```bash title="Next sequence number"
aptos move view \
    --function-id 0x1::multisig_account::next_sequence_number \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    "1"
  ]
}
```

</details>

### Enqueue a publication transaction

The first multisig transaction enqueued will be a transaction for publication of the [`CliArgs` example package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args).
First, generate a publication entry function JSON file:

```bash title="Command"
aptos move publish \
    --named-addresses test_account=$multisig_addr \
    --json-output-file publication.json
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "vm_status": "Publication entry function JSON file saved to publication.json"
  }
}
```

</details>

Now have Ace propose publication of the package from the multisig account, storing only the payload hash on-chain:

```bash title="Command"
aptos multisig create-transaction \
    --multisig-address $multisig_addr \
    --json-file publication.json \
    --hash-only \
    --private-key-file ace.key
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0xde5dfd2ca09cf2b3ca040386633de5c1c8aee5842d49303c757eb14819e20a3f",
    "gas_used": 510,
    "gas_unit_price": 100,
    "sender": "acee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
    "sequence_number": 1,
    "success": true,
    "timestamp_us": 1684645051403399,
    "version": 525305517,
    "vm_status": "Executed successfully"
  }
}
```

</details>

Note that the last resolved sequence number is still 0 because no transactions have been resolved:

```bash title="Last resolved sequence number"
aptos move view \
    --function-id 0x1::multisig_account::last_resolved_sequence_number \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    "0"
  ]
}
```

</details>

However the next sequence number has been incremented because a transaction has been enqueued:

```bash title="Next sequence number"
aptos move view \
    --function-id 0x1::multisig_account::next_sequence_number \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    "2"
  ]
}
```

</details>

The multisig transaction enqueued on-chain can now be inspected:

```bash title="Get transaction"
aptos move view \
    --function-id 0x1::multisig_account::get_transaction \
    --args \
        address:"$multisig_addr" \
        String:1
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    {
      "creation_time_secs": "1684645051",
      "creator": "0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
      "payload": {
        "vec": []
      },
      "payload_hash": {
        "vec": [
          "0xce31dac5c29fd54c643119b4011a4991bd96141a21be10100d75336230417e89"
        ]
      },
      "votes": {
        "data": [
          {
            "key": "0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
            "value": true
          }
        ]
      }
    }
  ]
}
```

</details>

Note from the above result that no payload is stored on-chain, and that Ace implicitly approved the transaction (voted `true`) upon the submission of the proposal.

### Enqueue a governance parameter transaction

Now have Bee enqueue a governance parameter setter transaction, storing the entire transaction payload on-chain:

```bash title="Command"
aptos multisig create-transaction \
    --multisig-address $multisig_addr \
    --function-id $multisig_addr::cli_args::set_vals \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args \
        u8:123 \
        "bool:[false, true, false, false]" \
        'address:[["0xace", "0xbee"], ["0xcad"], []]' \
    --private-key-file bee.key

```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x92c7f7c103f2f7409ec0ede1325ce69c9357dc07423d1801d2c49eeee74d91ae",
    "gas_used": 511,
    "gas_unit_price": 100,
    "sender": "bee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1684645156069617,
    "version": 525306308,
    "vm_status": "Executed successfully"
  }
}
```

</details>

Note the next sequence number has been incremented again:

```bash title="Next sequence number"
aptos move view \
    --function-id 0x1::multisig_account::next_sequence_number \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    "3"
  ]
}
```

</details>

Now both the publication and parameter transactions are pending:

```bash title="Get pending transactions"
aptos move view \
    --function-id 0x1::multisig_account::get_pending_transactions \
    --args \
        address:"$multisig_addr"
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    [
      {
        "creation_time_secs": "1684645051",
        "creator": "0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
        "payload": {
          "vec": []
        },
        "payload_hash": {
          "vec": [
            "0xce31dac5c29fd54c643119b4011a4991bd96141a21be10100d75336230417e89"
          ]
        },
        "votes": {
          "data": [
            {
              "key": "0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
              "value": true
            }
          ]
        }
      },
      {
        "creation_time_secs": "1684645156",
        "creator": "0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4",
        "payload": {
          "vec": [
            "0x002dc9b2fdba8ace3b9f96e5d3bb7ea04d39b1640020c8697eb0f1f4b33cad0d7708636c695f61726773087365745f76616c7302070000000000000000000000000000000000000000000000000000000000000001076163636f756e74074163636f756e740007000000000000000000000000000000000000000000000000000000000000000108636861696e5f696407436861696e49640003017b0504000100006403020000000000000000000000000000000000000000000000000000000000000ace0000000000000000000000000000000000000000000000000000000000000bee010000000000000000000000000000000000000000000000000000000000000cad00"
          ]
        },
        "payload_hash": {
          "vec": []
        },
        "votes": {
          "data": [
            {
              "key": "0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4",
              "value": true
            }
          ]
        }
      }
    ]
  ]
}
```

</details>

### Execute the publication transaction

Since only Ace has voted on the publication transaction (which he implicitly approved upon proposing) the transaction can't be executed yet:

```bash title="Can be executed"
aptos move view \
    --function-id 0x1::multisig_account::can_be_executed \
    --args \
        address:"$multisig_addr" \
        String:1
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    false
  ]
}
```

</details>

Before Bee votes, however, she checks that the payload hash stored on-chain matches the publication entry function JSON file:

```bash title="Checking transaction"
aptos multisig check-transaction \
    --multisig-address $multisig_addr \
    --json-file publication.json \
    --sequence-number 1
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "Status": "Transaction match",
    "Multisig transaction": {
      "creation_time_secs": "1684645051",
      "creator": "0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
      "payload": {
        "vec": []
      },
      "payload_hash": {
        "vec": [
          "0xce31dac5c29fd54c643119b4011a4991bd96141a21be10100d75336230417e89"
        ]
      },
      "votes": {
        "data": [
          {
            "key": "0xacee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
            "value": true
          }
        ]
      }
    }
  }
}
```

</details>

Since Bee has verified that the on-chain payload hash checks out against her locally-compiled package publication JSON file, she votes yes:


```bash title="Approving transaction"
aptos multisig approve \
    --multisig-address $multisig_addr \
    --sequence-number 1 \
    --private-key-file bee.key
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x24a12b1839b2dd114780289ec6e36a4a33b1ab2a4c3f22dd9512873aed65723a",
    "gas_used": 6,
    "gas_unit_price": 100,
    "sender": "bee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4",
    "sequence_number": 1,
    "success": true,
    "timestamp_us": 1684645251141034,
    "version": 525307001,
    "vm_status": "Executed successfully"
  }
}
```

</details>

Now the transaction can be executed:

```bash title="Can be executed"
aptos move view \
    --function-id 0x1::multisig_account::can_be_executed \
    --args \
        address:"$multisig_addr" \
        String:1
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    true
  ]
}
```

</details>

Now either Ace or Bee can invoke the publication transaction from the multisig account, passing the full transaction payload since only the hash was stored on-chain:

```bash title="Publication"
aptos multisig execute \
    --multisig-address $multisig_addr \
    --json-file publication.json \
    --private-key-file bee.key \
    --max-gas 10000
```

:::tip
Pending the resolution of [#8304](https://github.com/aptos-labs/aptos-core/issues/8304), the transaction simulator (which is used to estimate gas costs) is broken for multisig transactions, so you will have to manually specify a max gas amount.
:::

<details><summary>Output</summary>

Also pending the resolution of [#8304](https://github.com/aptos-labs/aptos-core/issues/8304), the CLI output for a successful multisig publication transaction execution results in an API error if only the payload hash has been stored on-chain, but the transaction can be manually verified using an explorer.

</details>

### Execute the governance parameter transaction

Since only Bee has voted on the governance parameter transaction (which she implicitly approved upon proposing), the transaction can't be executed yet:

```bash title="Can be executed"
aptos move view \
    --function-id 0x1::multisig_account::can_be_executed \
    --args \
        address:"$multisig_addr" \
        String:2
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    false
  ]
}
```

</details>

Before Ace votes, however, he checks that the payload stored on-chain matches the function arguments he expects:

```bash title="Checking transaction"
aptos multisig check-transaction \
    --multisig-address $multisig_addr \
    --function-id $multisig_addr::cli_args::set_vals \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args \
        u8:123 \
        "bool:[false, true, false, false]" \
        'address:[["0xace", "0xbee"], ["0xcad"], []]' \
    --sequence-number 2
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "Status": "Transaction match",
    "Multisig transaction": {
      "creation_time_secs": "1684645156",
      "creator": "0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4",
      "payload": {
        "vec": [
          "0x002dc9b2fdba8ace3b9f96e5d3bb7ea04d39b1640020c8697eb0f1f4b33cad0d7708636c695f61726773087365745f76616c7302070000000000000000000000000000000000000000000000000000000000000001076163636f756e74074163636f756e740007000000000000000000000000000000000000000000000000000000000000000108636861696e5f696407436861696e49640003017b0504000100006403020000000000000000000000000000000000000000000000000000000000000ace0000000000000000000000000000000000000000000000000000000000000bee010000000000000000000000000000000000000000000000000000000000000cad00"
        ]
      },
      "payload_hash": {
        "vec": []
      },
      "votes": {
        "data": [
          {
            "key": "0xbee090156aa0efa1fd6242d194400ef46471e2eca80dcd654532319c8b0355d4",
            "value": true
          }
        ]
      }
    }
  }
}
```

</details>

Note that the check fails if he modifies even a single argument:

```bash title="Checking transaction with modified u8"
aptos multisig check-transaction \
    --multisig-address $multisig_addr \
    --function-id $multisig_addr::cli_args::set_vals \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args \
        u8:200 \
        "bool:[false, true, false, false]" \
        'address:[["0xace", "0xbee"], ["0xcad"], []]' \
    --sequence-number 2
```

<details><summary>Output</summary>

```bash
{
  "Error": "Unexpected error: Payload mismatch"
}
```

</details>

Ace approves the transaction:

```bash title="Approving transaction"
aptos multisig approve \
    --multisig-address $multisig_addr \
    --sequence-number 2 \
    --private-key-file ace.key
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x5cdc4fd171d7b2ae3676b0c4a9a3fa1523ca46fde205ec17cc0ef7c0c92108d5",
    "gas_used": 6,
    "gas_unit_price": 100,
    "sender": "acee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
    "sequence_number": 2,
    "success": true,
    "timestamp_us": 1684646050747796,
    "version": 525312861,
    "vm_status": "Executed successfully"
  }
}
```

</details>

Since the payload was stored on-chain, it is not required to execute the pending transaction:

```bash title="Publication"
aptos multisig execute \
    --multisig-address $multisig_addr \
    --private-key-file ace.key \
    --max-gas 10000
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x2cc091926460ac37e0bff280d5cc6a3a225838ff8f13dc224be6cd5be6725fea",
    "gas_used": 505,
    "gas_unit_price": 100,
    "sender": "acee3447860cd5f14801badcbf69dbdb98a0c315999ded339bb9d3606ac4faa4",
    "sequence_number": 3,
    "success": true,
    "timestamp_us": 1684646121523835,
    "version": 525313412,
    "vm_status": "Executed successfully"
  }
}
```

</details>

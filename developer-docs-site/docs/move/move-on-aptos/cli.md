---
title: "Aptos Move CLI"
---

import CodeBlock from '@theme/CodeBlock';

# Use the Aptos Move CLI

The `aptos` tool is a command line interface (CLI) for developing on the Aptos blockchain, debugging, and for node operations. This document describes how to use the `aptos` CLI tool. To download or build the CLI, follow [Install Aptos CLI](../../tools/aptos-cli/install-cli/index.md).

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
In this case, see [Install the dependencies of Move Prover](../../tools/aptos-cli/install-cli/index.md#step-3-optional-install-the-dependencies-of-move-prover).

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
$ aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses hello_blockchain=8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71
```

:::tip
As an open source project, the source code as well as compiled code published to the Aptos blockchain is inherently open by default. This means code you upload may be downloaded from on-chain data. Even without source access, it is possible to regenerate Move source from Move bytecode. To disable source access, publish with the `--included-artifacts none` argument, like so:

```
aptos move publish --included-artifacts none
```
:::

You can additionally use named profiles for the addresses.  The first placeholder is `default`
```bash
$ aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses hello_blockchain=default
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

### Package info

This section references the [`CliArgs` example package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args), which contains the following manifest:

import move_toml from '!!raw-loader!../../../../aptos-move/move-examples/cli_args/Move.toml';

<CodeBlock language="toml" title="Move.toml">{move_toml}</CodeBlock>

Here, the package is deployed under the named address `test_account`.

:::tip
Set your working directory to [`aptos-move/move-examples/cli_args`](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/cli_args) to follow along:

```bash
cd <aptos-core-parent-directory>/aptos-core/aptos-move/move-examples/cli_args
```
:::

### Deploying the package

Start by mining a vanity address for Ace, who will deploy the package:


```bash title=Command
aptos key generate \
    --vanity-prefix 0xace \
    --output-file ace.key
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "Account Address:": "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "PublicKey Path": "ace.key.pub",
    "PrivateKey Path": "ace.key"
  }
}
```

</details>

:::tip
The exact account address should vary for each run, though the vanity prefix should not.
:::

Store Ace's address in a shell variable so you can call it inline later on:

```bash
# Your exact address should vary
ace_addr=0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46
```

Fund Ace's account with the faucet (either devnet or testnet):

```bash title=Command
aptos account fund-with-faucet --account $ace_addr
```

<details><summary>Output</summary>

```bash
{
  "Result": "Added 100000000 Octas to account acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46"
}
```

</details>

Now publish the package under Ace's account:

```bash title=Command
aptos move publish \
    --named-addresses test_account=$ace_addr \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x1d7b074dd95724c5459a1c30fe4cb3875e7b0478cc90c87c8e3f21381625bec1",
    "gas_used": 1294,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1685077849297587,
    "version": 528422121,
    "vm_status": "Executed successfully"
  }
}
```

</details>

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

:::tip
To pass vectors (including nested vectors) as arguments from the command line, use JSON syntax escaped with quotes!
:::

```bash title="Running function with nested vector arguments from CLI"
aptos move run \
    --function-id $ace_addr::cli_args::set_vals \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args \
        u8:123 \
        "bool:[false, true, false, false]" \
        'address:[["0xace", "0xbee"], ["0xcad"], []]' \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x5e141dc6c28e86fa9f5594de93d07a014264ebadfb99be6db922a929eb1da24f",
    "gas_used": 504,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 1,
    "success": true,
    "timestamp_us": 1685077888820037,
    "version": 528422422,
    "vm_status": "Executed successfully"
  }
}
```

</details>

The function ID, type arguments, and arguments can alternatively be specified in a JSON file:

import entry_json_file from '!!raw-loader!../../../../aptos-move/move-examples/cli_args/entry_function_arguments.json';

<CodeBlock language="json" title="entry_function_arguments.json">{entry_json_file}</CodeBlock>

Here, the call to `aptos move run` looks like:

```bash title="Running function with JSON input file"
aptos move run \
    --json-file entry_function_arguments.json \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x60a32315bb48bf6d31629332f6b1a3471dd0cb016fdee8d0bb7dcd0be9833e60",
    "gas_used": 3,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 2,
    "success": true,
    "timestamp_us": 1685077961499641,
    "version": 528422965,
    "vm_status": "Executed successfully"
  }
}
```

</details>

:::tip
If you are trying to run the example yourself don't forget to substitute Ace's actual address for `<test_account>` in `entry_function_arguments.json`!
:::

### View functions

Once the values in a `Holder` have been set, the `reveal()` view function can be used to check the first three fields, and to compare type arguments against the last two fields:

```rust title="View function"
:!: static/move-examples/cli_args/sources/cli_args.move view
```

This view function can be called with arguments specified either from the CLI or from a JSON file:

```bash title="Arguments via CLI"
aptos move view \
    --function-id $ace_addr::cli_args::reveal \
    --type-args \
        0x1::account::Account \
        0x1::account::Account \
    --args address:$ace_addr
```

```bash title="Arguments via JSON file"
aptos move view --json-file view_function_arguments.json
```

:::tip
If you are trying to run the example yourself don't forget to substitute Ace's actual address for `<test_account>` in `view_function_arguments.json` (twice)!
:::

import view_json_file from '!!raw-loader!../../../../aptos-move/move-examples/cli_args/view_function_arguments.json';

<CodeBlock language="json" title="view_function_arguments.json">{view_json_file}</CodeBlock>

```bash title="Output"
{
  "Result": [
    {
      "address_vec_vec": [
        [
          "0xace",
          "0xbee"
        ],
        [
          "0xcad"
        ],
        []
      ],
      "bool_vec": [
        false,
        true,
        false,
        false
      ],
      "type_info_1_match": true,
      "type_info_2_match": false,
      "u8_solo": 123
    }
  ]
}
```

### Script functions

The package also contains a script, `set_vals.move`, which is a wrapper for the setter function:

```rust title="script"
:!: static/move-examples/cli_args/scripts/set_vals.move script
```

First compile the package (this will compile the script):

```bash title=Compilation
aptos move compile --named-addresses test_account=$ace_addr
```

<details><summary>Output</summary>

```bash
{
  "Result": [
    "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46::cli_args"
  ]
}
```

</details>

Next, run `aptos move run-script`:

```bash title="Arguments via CLI"
aptos move run-script \
    --compiled-script-path build/CliArgs/bytecode_scripts/set_vals.mv \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args \
        u8:123 \
        "u8:[122, 123, 124, 125]" \
        address:"0xace" \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x1d644eba8187843cc43919469112339bc2c435a49a733ac813b7bc6c79770152",
    "gas_used": 3,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 3,
    "success": true,
    "timestamp_us": 1685078415935612,
    "version": 528426413,
    "vm_status": "Executed successfully"
  }
}
```

</details>

```bash title="Arguments via JSON file"
aptos move run-script \
    --compiled-script-path build/CliArgs/bytecode_scripts/set_vals.mv \
    --json-file script_function_arguments.json \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x840e2d6a5ab80d5a570effb3665f775f1755e0fd8d76e52bfa7241aaade883d7",
    "gas_used": 3,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 4,
    "success": true,
    "timestamp_us": 1685078516832128,
    "version": 528427132,
    "vm_status": "Executed successfully"
  }
}
```

</details>

import script_json_file from '!!raw-loader!../../../../aptos-move/move-examples/cli_args/script_function_arguments.json';

<CodeBlock language="json" title="script_function_arguments.json">{script_json_file}</CodeBlock>

Both such script function invocations result in the following `reveal()` view function output:

```bash title="View function call"
aptos move view \
    --function-id $ace_addr::cli_args::reveal \
    --type-args \
        0x1::account::Account \
        0x1::chain_id::ChainId \
    --args address:$ace_addr
```

```json title="View function output"
{
  "Result": [
    {
      "address_vec_vec": [
        [
          "0xace"
        ]
      ],
      "bool_vec": [
        false,
        false,
        true,
        true
      ],
      "type_info_1_match": true,
      "type_info_2_match": true,
      "u8_solo": 123
    }
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
If you would like to follow along, start by completing the [Arguments in JSON](#arguments-in-json) tutorial steps!
:::

For this example, Ace and Bee will conduct governance operations from a 2-of-2 "multisig v2" account (an on-chain multisig account per [`multisig_account.move`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/multisig_account.move))

### Account creation

Since Ace's account was created during the [Arguments in JSON](#arguments-in-json) tutorial, start by mining a vanity address account for Bee too:

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
    "Account Address:": "0xbeec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc"
  }
}
```

</details>

:::tip
The exact account address should vary for each run, though the vanity prefix should not.
:::

Store Bee's address in a shell variable so you can call it inline later on:

```bash
# Your exact address should vary
bee_addr=0xbeec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc
```

Fund Bee's account using the faucet:

```bash title=Command
aptos account fund-with-faucet --account $bee_addr
```

<details><summary>Output</summary>

```bash
{
  "Result": "Added 100000000 Octas to account beec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc"
}
```

</details>

Ace can now create a multisig account:

```bash title=Command
aptos multisig create \
    --additional-owners $bee_addr \
    --num-signatures-required 2 \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "multisig_address": "57478da34604655c68b1dcb89e4f4a9124b6c0ecc1c59a0931d58cc4e60ac5c5",
    "transaction_hash": "0x849cc756de2d3b57210f5d32ae4b5e7d1f80e5d376233885944b6f3cc2124a05",
    "gas_used": 1524,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 5,
    "success": true,
    "timestamp_us": 1685078644186194,
    "version": 528428043,
    "vm_status": "Executed successfully"
  }
}
```

</details>

Store the multisig address in a shell variable:

```bash
# Your address should vary
multisig_addr=0x57478da34604655c68b1dcb89e4f4a9124b6c0ecc1c59a0931d58cc4e60ac5c5
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
      "0xbeec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc",
      "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46"
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
First, generate a publication payload entry function JSON file:

```bash title="Command"
aptos move build-publish-payload \
    --named-addresses test_account=$multisig_addr \
    --json-output-file publication.json \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": "Publication payload entry function JSON file saved to publication.json"
}
```

</details>

Now have Ace propose publication of the package from the multisig account, storing only the payload hash on-chain:

```bash title="Command"
aptos multisig create-transaction \
    --multisig-address $multisig_addr \
    --json-file publication.json \
    --store-hash-only \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x70c75903f8e1b1c0069f1e84ef9583ad8000f24124b33a746c88d2b031f7fe2c",
    "gas_used": 510,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 6,
    "success": true,
    "timestamp_us": 1685078836492390,
    "version": 528429447,
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
      "creation_time_secs": "1685078836",
      "creator": "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
      "payload": {
        "vec": []
      },
      "payload_hash": {
        "vec": [
          "0x62b91159c1428c1ef488c7290771de458464bd665691d9653d195bc28e0d2080"
        ]
      },
      "votes": {
        "data": [
          {
            "key": "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
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
    --private-key-file bee.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0xd0a348072d5bfc5a2e5d444f92f0ecc10b978dad720b174303bc6d91342f27ec",
    "gas_used": 511,
    "gas_unit_price": 100,
    "sender": "beec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1685078954841650,
    "version": 528430315,
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
        "creation_time_secs": "1685078836",
        "creator": "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
        "payload": {
          "vec": []
        },
        "payload_hash": {
          "vec": [
            "0x62b91159c1428c1ef488c7290771de458464bd665691d9653d195bc28e0d2080"
          ]
        },
        "votes": {
          "data": [
            {
              "key": "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
              "value": true
            }
          ]
        }
      },
      {
        "creation_time_secs": "1685078954",
        "creator": "0xbeec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc",
        "payload": {
          "vec": [
            "0x0057478da34604655c68b1dcb89e4f4a9124b6c0ecc1c59a0931d58cc4e60ac5c508636c695f61726773087365745f76616c7302070000000000000000000000000000000000000000000000000000000000000001076163636f756e74074163636f756e740007000000000000000000000000000000000000000000000000000000000000000108636861696e5f696407436861696e49640003017b0504000100006403020000000000000000000000000000000000000000000000000000000000000ace0000000000000000000000000000000000000000000000000000000000000bee010000000000000000000000000000000000000000000000000000000000000cad00"
          ]
        },
        "payload_hash": {
          "vec": []
        },
        "votes": {
          "data": [
            {
              "key": "0xbeec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc",
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

Before Bee votes, however, she verifies that the payload hash stored on-chain matches the publication entry function JSON file:

```bash title="Verifying transaction proposal"
aptos multisig verify-proposal \
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
      "creation_time_secs": "1685078836",
      "creator": "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
      "payload": {
        "vec": []
      },
      "payload_hash": {
        "vec": [
          "0x62b91159c1428c1ef488c7290771de458464bd665691d9653d195bc28e0d2080"
        ]
      },
      "votes": {
        "data": [
          {
            "key": "0xacef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
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
    --private-key-file bee.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0xa5fb49f1077de6aa6d976e6bcc05e4c50c6cd061f1c87e8f1ea74e7a04a06bd1",
    "gas_used": 6,
    "gas_unit_price": 100,
    "sender": "beec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc",
    "sequence_number": 1,
    "success": true,
    "timestamp_us": 1685079892130861,
    "version": 528437204,
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
aptos multisig execute-with-payload \
    --multisig-address $multisig_addr \
    --json-file publication.json \
    --private-key-file bee.key \
    --max-gas 10000 \
    --assume-yes
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

Before Ace votes, however, he verifies that the payload stored on-chain matches the function arguments he expects:

```bash title="Verifying transaction proposal"
aptos multisig verify-proposal \
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
      "creation_time_secs": "1685078954",
      "creator": "0xbeec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc",
      "payload": {
        "vec": [
          "0x0057478da34604655c68b1dcb89e4f4a9124b6c0ecc1c59a0931d58cc4e60ac5c508636c695f61726773087365745f76616c7302070000000000000000000000000000000000000000000000000000000000000001076163636f756e74074163636f756e740007000000000000000000000000000000000000000000000000000000000000000108636861696e5f696407436861696e49640003017b0504000100006403020000000000000000000000000000000000000000000000000000000000000ace0000000000000000000000000000000000000000000000000000000000000bee010000000000000000000000000000000000000000000000000000000000000cad00"
        ]
      },
      "payload_hash": {
        "vec": []
      },
      "votes": {
        "data": [
          {
            "key": "0xbeec980219d246581cef5166dc6ba5fb1e090c7a7786a5176d111a9029b16ddc",
            "value": true
          }
        ]
      }
    }
  }
}
```

</details>

Note that the verification fails if he modifies even a single argument:

```bash title="Failed transaction verification with modified u8"
aptos multisig verify-proposal \
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
  "Error": "Unexpected error: Transaction mismatch: The transaction you provided has a payload hash of 0xe494b0072d6f940317344967cf0e818c80082375833708c773b0275f3ad07e51, but the on-chain transaction proposal you specified has a payload hash of 0x070ed7c3f812f25f585461305d507b96a4e756f784e01c8c59901871267a1580. For more info, see https://aptos.dev/move/move-on-aptos/cli#multisig-governance"
}
```

</details>

Ace approves the transaction:

```bash title="Approving transaction"
aptos multisig approve \
    --multisig-address $multisig_addr \
    --sequence-number 2 \
    --private-key-file ace.key \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0x233427d95832234fa13dddad5e0b225d40168b4c2c6b84f5255eecc3e68401bf",
    "gas_used": 6,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 7,
    "success": true,
    "timestamp_us": 1685080266378400,
    "version": 528439883,
    "vm_status": "Executed successfully"
  }
}
```

</details>

Since the payload was stored on-chain, it is not required to execute the pending transaction:

```bash title="Execution"
aptos multisig execute \
    --multisig-address $multisig_addr \
    --private-key-file ace.key \
    --max-gas 10000 \
    --assume-yes
```

<details><summary>Output</summary>

```bash
{
  "Result": {
    "transaction_hash": "0xbc99f929708a1058b223aa880d04607a78ebe503367ec4dab23af4a3bdb541b2",
    "gas_used": 505,
    "gas_unit_price": 100,
    "sender": "acef1b9b7d4ab208b99fed60746d18dcd74865edb7eb3c3f1428233988e4ba46",
    "sequence_number": 8,
    "success": true,
    "timestamp_us": 1685080344045461,
    "version": 528440423,
    "vm_status": "Executed successfully"

```

</details>

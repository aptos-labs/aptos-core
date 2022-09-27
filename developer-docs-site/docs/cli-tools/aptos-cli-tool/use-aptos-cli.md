---
title: "Using Aptos CLI"
id: "use-aptos-cli"
---

# Using Aptos CLI

The `aptos` tool is a command line interface (CLI) for developing on the Aptos blockchain, debugging, and for node operations. This document describes how to use the `aptos` CLI tool. To install the CLI, see [Install Aptos CLI](install-aptos-cli).

## Command line help

Command line help is available.  Type `aptos help` or `aptos --help` to see the available command options. See below the usage output from `aptos --help`: 

```bash
USAGE:
    aptos <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    account       Tool for interacting with accounts
    config        Tool for interacting with configuration of the Aptos CLI tool
    genesis       Tool for setting up an Aptos chain Genesis transaction
    governance    Tool for on-chain governance
    help          Print this message or the help of the given subcommand(s)
    info          Show build information about the CLI
    init          Tool to initialize current directory for the aptos tool
    key           Tool for generating, inspecting, and interacting with keys
    move          Tool for Move related operations
    node          Tool for operations related to nodes
    stake         Tool for manipulating stake
```

### Command-specific help

Command-specific help is also available.  For example, see below the usage output from `aptos move --help`:

```bash

USAGE:
    aptos move <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    clean
            Cleans derived artifacts of a package
    compile
            Compiles a package and returns the [`ModuleId`]s
    download
            Downloads a package and stores it in a directory named after the package
    help
            Print this message or the help of the given subcommand(s)
    init
            Creates a new Move package at the given location
    list
            Lists information about packages and modules on-chain
    prove
            Proves the Move package
    publish
            Publishes the modules in a Move package to the Aptos blockchain
    run
            Run a Move function
    test
            Runs Move unit tests for a package
    transactional-test
            Run Move transactional tests
```
### Sub-command help

Help for sub-commands is also available.  For example, see below the usage output from `aptos move compile --help`:

```bash

USAGE:
    aptos move compile [OPTIONS]

OPTIONS:
    -h, --help
            Print help information

        --named-addresses <NAMED_ADDRESSES>
            Named addresses for the move binary
            
            Example: alice=0x1234, bob=0x5678
            
            Note: This will fail if there are duplicates in the Move.toml file remove those first.
            
            [default: ]

        --output-dir <OUTPUT_DIR>
            Path to save the compiled move package
            
            Defaults to `<package_dir>/build`

        --package-dir <PACKAGE_DIR>
            Path to a move package (the folder with a Move.toml file)

    -V, --version
            Print version information
```

## CLI information

Run the `aptos info` command to get the CLI information for debugging purposes. See an example output of the `aptos info` command:

```bash
{
  "Result": {
    "build_branch": "testnet",
    "build_cargo_version": "cargo 1.62.1 (a748cf5a3 2022-06-08)",
    "build_commit_hash": "f8bf8fdeec33c8c6ff3d1cbaf4990b9e54c2176a",
    "build_os": "macos-x86_64",
    "build_pkg_version": "0.3.2",
    "build_rust_channel": "1.62.1-x86_64-apple-darwin",
    "build_rust_version": "rustc 1.62.1 (e092d0b6b 2022-07-16)",
    "build_tag": "",
    "build_time": "2022-08-26 22:27:31 +00:00"
  }
}
```

## Configuration examples

Configuration for the CLI works like this:

### In the current working directory for local runs

1. Your configurations are in a **local** YAML configuration file `.aptos/config.yaml`, i.e., located in the current working directory where you run the CLI. In this case you must run your CLI commands from this current working directory for this configuration to be used. 
2. You can verify that the CLI is set to use this local configuration YAML file by running the command: 

  ```bash
  aptos config show-global-config
  ```
  You should see the below output:
  ```bash
  {
    "Result": {
      "config_type": "Workspace"
    }
  }
  ```
  The `Workspace` value for the `config_type` indicates that the `.aptos/config.yaml` file is used for the CLI configuration.

### In the home directory for the global runs

1. Your configurations are in a **global** YAML configuration file `~/.aptos/global_config.yaml`, i.e., located in your home directory.
2. Set the CLI to use this global configuration YAML file by running this command:
  ```bash
  aptos config set-global-config --config-type global
  ```
  You will see the below output:
  ```
  {
    "Result": {
      "config_type": "Global"
    }
  }
  ```
  You can also show the global configuration with the `show-global-config` command.
  ```bash
  $ aptos config show-global-config
  {
    "Result": {
      "config_type": "Global"
    }
  }
  ```

:::tip Default configuration
If you did not set any global configuration, then the `./.aptos/config.yaml` in the current working directory is used for configuration.
:::

### Setting up shell completion

You can set up shell completions with the `generate-shell-completions` command.  You can lookup configuration for your specific shell. The supported shells are `[bash, zsh, fish, powershell, elvish]`. An example is below for [`oh my zsh`](https://ohmyz.sh/).

```bash
aptos config generate-shell-completions --shell zsh --output-file ~/.oh-my-zsh/completions/_aptos
```

## Initialize local configuration and create an account

A local folder named `.aptos/` will be created with a configuration `config.yaml` which can be used to store configuration between CLI runs.  This is local to your run, so you will need to continue running CLI from this folder, or reinitialize in another folder.

### Step 1: Run Aptos init

The `aptos init` command will initialize the configuration with the private key you provided.

```bash
$ aptos init
Configuring for profile default
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com]

No rest url given, using https://fullnode.devnet.aptoslabs.com...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696 doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696!  Run `aptos help` for more information about commands

{
  "Result": "Success"
}
```

### Step 2: Changing the configuration

To change the configuration, you can either run the command `aptos init` or you can manually edit the `.aptos/config.yaml` that is in your current working directory.

### Creating other profiles

You can also create other profiles for different endpoints and different keys.  These can be made by adding the `--profile` argument, and can be used in most other commands to replace command line arguments.

```bash
$ aptos init --profile superuser
Configuring for profile superuser
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com]

No rest url given, using https://fullnode.devnet.aptoslabs.com...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account 18B61497FD290B02BB0751F44381CADA1657C2B3AA6194A00D9BC9A85FAD3B04 doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account 18B61497FD290B02BB0751F44381CADA1657C2B3AA6194A00D9BC9A85FAD3B04!  Run `aptos help` for more information about commands
{
  "Result": "Success"
}
```

## Account examples

### Fund an account with the faucet

You can fund an account with the faucet via the CLI by using either an account address or with `default` (which defaults to the account address created with `aptos init`). 

For example, to fund the account `00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696` that was created above with the `aptos init` command:

```bash
$ aptos account fund-with-faucet --account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696
{
  "Result": "Added 10000 coins to account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696"
}
```

```bash
$ aptos account fund-with-faucet --account default
{
  "Result": "Added 10000 coins to account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696"
}
```

### View an account's balance and transfer events

You can view the balance and transfer events (deposits and withdrawals) either by explicity specying the accound address, as below:

```bash
$ aptos account list --query balance --account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696
```
or by specifying the `default` as below:
```bash
$ aptos account list --query balance --account default
```

Both the above commands will generate the following information on your terminal:

```bash
{
  "Result": [
    {
      "coin": {
        "value": "110000"
      },
      "deposit_events": {
        "counter": "3",
        "guid": {
          "id": {
            "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
            "creation_num": "2"
          }
        }
      },
      "frozen": false,
      "withdraw_events": {
        "counter": "0",
        "guid": {
          "id": {
            "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
            "creation_num": "3"
          }
        }
      }
    }
  ]
}
```

### Listing resources in an account

You can list the resources in an account from the command line. For example, see below for how to list the resources in the account you just created above:

```bash
$ aptos account list --query resources --account default
```
or
```bash
$ aptos account list --query resources --account 0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696
```

Both the above commands will generate the following resource list information on your terminal:

```bash
{
  "Result": [
    {
      "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>": {
        "coin": {
          "value": "110000"
        },
        "deposit_events": {
          "counter": "3",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "2"
            }
          }
        },
        "frozen": false,
        "withdraw_events": {
          "counter": "0",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "3"
            }
          }
        }
      }
    },
    {
      "0x1::account::Account": {
        "authentication_key": "0x00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
        "coin_register_events": {
          "counter": "1",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "0"
            }
          }
        },
        "guid_creation_num": "4",
        "key_rotation_events": {
          "counter": "0",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "1"
            }
          }
        },
        "rotation_capability_offer": {
          "for": {
            "vec": []
          }
        },
        "sequence_number": "0",
        "signer_capability_offer": {
          "for": {
            "vec": []
          }
        }
      }
    }
  ]
}
```

### List the default profile

You can also list the default profile from configuration with no account specified.

:::tip 
Account addresses may differ from example to example in this section.
:::

```bash
$ aptos account list
{
  "Result": [
    {
      "coin": {
        "value": "10000"
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
        "counter": "0",
        "guid": {
          "id": {
            "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
            "creation_num": "2"
          }
        }
      }
    },
    {
      "register_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
            "creation_num": "0"
          }
        }
      }
    },
    {
      "counter": "3"
    },
    {
      "authentication_key": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
      "self_address": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
      "sequence_number": "0"
    }
  ]
}
```

### Use the name of the profile

Additionally, any place that takes an account can use the name of a profile:

```bash
$ aptos account list --query resources --account superuser
{
  "Result": [
    {
      "coin": {
        "value": "10000"
      },
      "deposit_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
            "creation_num": "1"
          }
        }
      },
      "withdraw_events": {
        "counter": "0",
        "guid": {
          "id": {
            "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
            "creation_num": "2"
          }
        }
      }
    },
    {
      "register_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
            "creation_num": "0"
          }
        }
      }
    },
    {
      "counter": "3"
    },
    {
      "authentication_key": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
      "self_address": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
      "sequence_number": "0"
    }
  ]
}
```

### Listing modules in an account

You can pass different types of queries to view different items under an account. Currently, 'resources' and
'modules' are supported but more query types are coming. For example, to fetch modules:
```bash
$ aptos account list --query modules                   
{
  "Result": [
    {
      "bytecode": "0xa11ceb0b050000000b01000a020a12031c2504410405452d0772da0108cc0240068c030a0a9603150cab03650d90040400000101010201030104000506000006080004070700020e0401060100080001000009020300010f0404000410060100031107000002120709010602130a030106050806080105010802020c0a02000103040508020802070801010a0201060c010800010b0301090002070b030109000900074d657373616765056572726f72056576656e74067369676e657206737472696e67124d6573736167654368616e67654576656e740d4d657373616765486f6c64657206537472696e670b6765745f6d6573736167650b7365745f6d6573736167650c66726f6d5f6d6573736167650a746f5f6d657373616765076d657373616765156d6573736167655f6368616e67655f6576656e74730b4576656e7448616e646c65096e6f745f666f756e6404757466380a616464726573735f6f66106e65775f6576656e745f68616e646c650a656d69745f6576656e74b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb0000000000000000000000000000000000000000000000000000000000000001030800000000000000000002020a08020b08020102020c08020d0b030108000001000101030b0a002901030607001102270b002b0110001402010104010105240b0111030c040e0011040c020a02290120030b05120e000b040e00380012012d0105230b022a010c050a051000140c030a050f010b030a04120038010b040b050f0015020100010100",
      "abi": {
        "address": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "name": "Message",
        "friends": [],
        "exposed_functions": [
          {
            "name": "get_message",
            "visibility": "public",
            "is_entry": false,
            "generic_type_params": [],
            "params": [
              "address"
            ],
            "return": [
              "0x1::string::String"
            ]
          },
          {
            "name": "set_message",
            "visibility": "public",
            "is_entry": true,
            "generic_type_params": [],
            "params": [
              "signer",
              "vector<u8>"
            ],
            "return": []
          }
        ],
        "structs": [
          {
            "name": "MessageChangeEvent",
            "is_native": false,
            "abilities": [
              "drop",
              "store"
            ],
            "generic_type_params": [],
            "fields": [
              {
                "name": "from_message",
                "type": "0x1::string::String"
              },
              {
                "name": "to_message",
                "type": "0x1::string::String"
              }
            ]
          },
          {
            "name": "MessageHolder",
            "is_native": false,
            "abilities": [
              "key"
            ],
            "generic_type_params": [],
            "fields": [
              {
                "name": "message",
                "type": "0x1::string::String"
              },
              {
                "name": "message_change_events",
                "type": "0x1::event::EventHandle<0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb::Message::MessageChangeEvent>"
              }
            ]
          }
        ]
      }
    }
  ]
}
```

### Transferring coins

The Aptos CLI is a simple wallet as well, and can transfer coins between accounts.
```bash
$ aptos account transfer --account superuser --amount 100
{
  "Result": {
    "gas_used": 73,
    "balance_changes": {
      "742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc": {
        "coin": {
          "value": "10100"
        },
        "deposit_events": {
          "counter": "2",
          "guid": {
            "id": {
              "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
              "creation_num": "1"
            }
          }
        },
        "withdraw_events": {
          "counter": "0",
          "guid": {
            "id": {
              "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
              "creation_num": "2"
            }
          }
        }
      },
      "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb": {
        "coin": {
          "value": "9827"
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
      }
    },
    "sender": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
    "success": true,
    "version": 1139,
    "vm_status": "Executed successfully"
  }
}
```

## Key examples

### Generating a key

To allow generating private keys, you can use the `aptos key generate command`.  You can generate
either `x25519` or `ed25519` keys.
```bash
$ aptos key generate --key-type ed25519 --output-file output.key
{
  "Result": {
    "PrivateKey Path": "output.key",
    "PublicKey Path": "output.key.pub"
  }
}
```

### Generating a peer config

To allow others to connect to your node, you need to generate a peer configuration. Below command shows how you can use
the `aptos` CLI to generate a peer configuration and write it into a file named `peer_config.yaml`.
```bash
$ aptos key extract-peer --output-file peer_config.yaml
```

The above command will generate the following output on the terminal:
```bash
{
  "Result": {
    "8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752": {
      "addresses": [],
      "keys": [
        "0x8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752"
      ],
      "role": "Upstream"
    }
  }
}
```

The `peer_config.yaml` file will be created in your current working directory, with the contents as shown in the below example:
```bash
---
8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752:
  addresses: []
  keys:
    - "0x8cfb85603080b13013b57e2e80887c695cfecd7ad8217d1cac22fa6f3b0b5752"
  role: Upstream
```

**Note:** In the addresses key, you should fill in your address.

## Move examples

### Compiling Move

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

### Compiling and unit testing Move

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

### Proving Move

The `aptos` CLI can be used to run [Move Prover](https://github.com/move-language/move/tree/main/language/move-prover) which is a formal verification tool for the Move language. The below example proves the `hello_prover` package in [move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples).
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
In this case, see [Install the dependencies of Move Prover](install-aptos-cli#step-3-optional-install-the-dependencies-of-move-prover).

### Debug and print stack trace

In this example, we will use `DebugDemo` in [debug-move-example](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos/debug-move-example).

First, you need to include Move nursery in your Move [toml file](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos/debug-move-example/Move.toml).

Now, you can use `Debug::print` and `Debug::print_stack_trace` in your [DebugDemo Move file](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos/debug-move-example/sources/DebugDemo.move).

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


### Publishing a Move package with a named address

In this example, we'll use the `HelloBlockchain` in [move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples).

Publish the package with your account address set for `HelloBlockchain`.

Here, you need to change 8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71 to your account address.
```bash
$ aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71
```

You can additionally use named profiles for the addresses.  The first placeholder is `default`
```bash
$ aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=default
```

:::tip
When publishing Move modules, if multiple modules are in one package, then all the modules in this package must have the same account. If they have different accounts, then the publishing will fail at the transaction level. 
:::

### Running a Move function

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

## Node command examples

### Running a local testnet

You can run a local testnet from the aptos CLI, that will match the version it was built with.  Additionally, it can
run a faucet side by side with the local single node testnet.

```bash
$ aptos node run-local-testnet --with-faucet
Completed generating configuration:
        Log file: "/Users/greg/.aptos/testnet/validator.log"
        Test dir: "/Users/greg/.aptos/testnet"
        Aptos root key path: "/Users/greg/.aptos/testnet/mint.key"
        Waypoint: 0:d302c6b10e0fa68bfec9cdb383f24ef1189d8850d50b832365eea21ae52d8101
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        Fullnode network: /ip4/0.0.0.0/tcp/6181

Aptos is running, press ctrl-c to exit
```

This will have consistent state if the node is shutdown, it will start with the previous state.
If you want to restart the chain from genesis, you can add the `--force-restart` flag.

```bash
$ aptos node run-local-testnet --with-faucet --force-restart
Are you sure you want to delete the existing chain? [yes/no] >
yes
Completed generating configuration:
        Log file: "/Users/greg/.aptos/testnet/validator.log"
        Test dir: "/Users/greg/.aptos/testnet"
        Aptos root key path: "/Users/greg/.aptos/testnet/mint.key"
        Waypoint: 0:649efc34c813d0db8db6fa5b1ffc9cc62f726bb5168e7f4b8730bb155d6213ea
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        Fullnode network: /ip4/0.0.0.0/tcp/6181

Aptos is running, press ctrl-c to exit
```

## Genesis ceremonies

The `aptos` tool supports bootstrapping new blockchains through what is known as a genesis ceremony. The output of the genesis ceremony is the output of move instructions that prepares a blockchain for online operation. The input consists of:

* A set of validators and their configuration
* The initial set of Move modules, known as a framework
* A unique `ChainId` (u8) that distinguishes this from other deployments
* For test chains, there also exists an account that manages the minting of AptosCoin

## Generating genesis

* The genesis organizer constructs a `Layout` and distributes it.
* The genesis organizer prepares the Aptos framework's bytecode and distributes it.
* Each participant generates their `ValidatorConfiguration` and distributes it.
* Each participant generates a `genesis.blob` from the resulting contributions
* The genesis organizer executes the `genesis.blob` to derive the initial waypoint and distributes it.
* Each participant begins their `aptos-node`. The `aptos-node` verifies upon startup that the `genesis.blob` with the waypoint provided by the genesis organizer .
* The blockchain will begin consensus after a quorum of stake is available.

### Prepare aptos-core

The following guide assumes that you have access to the Aptos-core repository or the associated tools. You can download and prepare Aptos-core from [GitHub](https://github.com/aptos-labs/aptos-core):

```
git clone https://github.com/aptos-labs/aptos-core.git
cd aptos-core
git checkout --track origin/testnet
./scripts/dev_setup.sh
source ~/.cargo/env
```

### The `layout` file

The layout file contains:
* `root_key`: an Ed25519 public key for AptosCoin management.
* `users`: the set of participants
* `chain_id`: the `ChainId` or a unique integer that distinguishes this deployment from other Aptos networks

An example:
```
root_key: "0xca3579457555c80fc7bb39964eb298c414fd60f81a2f8eedb0244ec07a26e575"
users:
  - alice
  - bob
chain_id: 8
```

### Building the Aptos Framework

From your Aptos-core repository, build the framework and package it:
```
cargo run --package framework
mkdir aptos-framework-release
cp aptos-framework/releases/artifacts/current/build/**/bytecode_modules/* aptos-framework-release
```

The framework will be stored within the `aptos-framework-release` directory.

### The `ValidatorConfiguration` file

The `ValidatorConfiguration` file contains:

* `account_address`: The account that manages this validator. This must be derived from the `account_key` provided within te `ValidatorConfiguration` file.
* `consensus_key`: The public key for authenticating consensus messages from the validator
* `account_key`: The public key for the account that manages this validator. This is used to derive the `account_address`.
* `network_key`: The public key for both validator and fullnode network authentication and encryption.
* `validator_host`: The network address where the validator resides. This contains a `host` and `port` field. The `host` should either be a DNS name or an IP address. Currently only IPv4 is supported.
* `full_node_host`: An optional network address where the fullnode resides. This contains a `host` and `port` field. The `host` should either be a DNS name or an IP address. Currently only IPv4 is supported.
* `stake_amount`: The number of coins being staked by this node. This is expected to be `1`, if it is different the configuration will be considered invalid.

An example:
```
account_address: ccd49f3ea764365ac21e99f029ca63a9b0fbfab1c8d8d5482900e4fa32c5448a
consensus_key: "0xa05b8f41057ac72f9ca99f5e3b1b787930f03ba5e448661f2a1fac98371775ee"
account_key: "0x3d15ab64c8b14c9aab95287fd0eb894aad0b4bd929a5581bcc8225b5688f053b"
network_key: "0x43ce1a4ac031b98bb1ee4a5cd72a4cca0fd72933d64b22cef4f1a61895c2e544"
validator_host:
  host: bobs_host
  port: 6180
full_node_host:
  host: bobs_host
  port: 6182
stake_amount: 1
```

To generate this using the `aptos` CLI:

1. Generate your validator's keys:

```
cargo run --package aptos -- genesis generate-keys --output-dir bobs
```

2. Generate your `ValidatorConfiguration`:

```
cargo run --package aptos -- \\
    genesis set-validator-configuration \\
    --keys-dir bobs \\
    --username bob \\
    --validator-host bobs_host:6180 \\
    --full-node-host bobs_host:6180 \\
    --local-repository-dir .
```

3. The last command will produce a `bob.yaml` file that should be distributed to other participants for `genesis.blob` generation.

### Generating a genesis and waypoint

`genesis.blob` and the waypoint can be generated after obtaining the `Layout` file, each of the individual `ValidatorConfiguration` files, and the framework release. It is important to validate that the `ValidatorConfiguration` provided in the earlier stage is the same as in the distribution for generating the `genesis.blob`. If there is a mismatch, inform all participants.

To generate the `genesis.blob` and waypoint:
* Place the `Layout` file in a directory, e.g., `genesis`.
* Place all the `ValidatorConfiguration` files into the `genesis` directory.
* Ensure that the `ValidatorConfiguration` files are listed under the set of `users` within the `Layout` file.
* Make a `framework` directory within the `genesiss` directory and place the framework release `.mv` files into the `framework` directory.
* Use the `aptos` CLI to generate genesis and waypoint:

```
cargo run --package aptos -- genesis generate-genesis --local-repository-dir genesis
```

### Starting an `aptos-node`

Upon generating the `genesis.blob` and waypoint, place them into your validator and fullnode's configuration directory and begin your validator and fullnode.

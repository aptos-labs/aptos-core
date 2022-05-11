# Aptos Command Line Interface (CLI) Tool

The `aptos` tool is a command line interface (CLI) for debugging, development, and node operation.
This document describes how to install the `aptos` CLI tool and how to use it.
## Installation
### Install precompiled binary (easy mode)
* Navigate to the [release page](https://github.com/aptos-labs/aptos-core/releases) for Aptos CLI.
* Download the latest release for your computer.
* Place this at a location for you to run it e.g. `~/bin/aptos` in Linux.
* On Linux and Mac, make this executable `chmod +x ~/bin/aptos`.
* Now type `~/bin/aptos help` to read help instructions.
* If you want you can add `~/bin` to your path in your appropriate `.bashrc` or `.zshrc` for future use

### Install Cargo (harder mode)
You will need the `cargo` package manager to install the `aptos` CLI tool.  Follow the below steps.
1. Follow the `cargo` [installation instructions on this page](https://doc.rust-lang.org/cargo/getting-started/installation.html)
   and install `cargo`.  Proceed only after you successfully install `cargo`.
2. Execute the below step to ensure that your current shell environment knows where `cargo` is.
```bash
source $HOME/.cargo/env
```
### Install the `aptos` CLI
1. Install the `aptos` CLI tool by running the below command.  You can run this command from any directory.  The `aptos`
   CLI tool will be installed into your `CARGO_HOME`, usually `~/.cargo`:
```bash
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos
```
2. Confirm that the `aptos` CLI tool is installed successfully by running the below command.  The terminal will display
   the path to the `aptos` CLI's location.
```bash
which aptos
```

## Using the `aptos` CLI
### Command Line Help
Command line help is available.  Type `aptos help` or `aptos --help` to see the available command options.
```bash
$ aptos help
aptos 0.1.0
Aptos Labs <opensource@aptoslabs.com>
CLI tool for interacting with the Aptos blockchain and nodes

USAGE:
    aptos <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    account    CLI tool for interacting with accounts
    config     Tool for configuration of the CLI tool
    genesis    Tool for setting up and building the Genesis transaction
    help       Print this message or the help of the given subcommand(s)
    init       Tool to initialize current directory for the aptos tool
    key        CLI tool for generating, inspecting, and interacting with keys
    move       CLI tool for performing Move tasks
```

Command specific help is also available.  For example, type `aptos move --help` to get command-specific help.
```bash
$ aptos move --help
aptos-move 0.1.0
CLI tool for performing Move tasks

USAGE:
    aptos move <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    compile    Compiles a package and returns the [`ModuleId`]s
    help       Print this message or the help of the given subcommand(s)
    init       Creates a new Move package at the given location
    publish    Publishes the modules in a Move package
    run        Run a Move function
    test       Run Move unit tests against a package path
```

Help for sub-commands is also available.  For example, type `aptos move compile --help` to get command-specific help.
```bash
$ aptos move compile --help
aptos-move-compile 0.1.0
Compiles a package and returns the [`ModuleId`]s

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

            [default: .]

    -V, --version
            Print version information
```

## Examples

### Initialize local configuration and create an account

A local folder named `.aptos/` will be created with a configuration `config.yaml` which can be used
to store configuration between CLI runs.  This is local to your run, so you will need to continue running CLI from this
folder, or reinitialize in another folder.

#### Step 1) Run Aptos init

This will initialize the configuration with the private key given.
```bash
$ aptos init
Configuring for profile default
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com]

No rest url given, using https://fullnode.devnet.aptoslabs.com...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account 50A49D913AA6381C01579E3FC00784B49AFA3A771F06389EBC65F8FF3A4E9A7D doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account 50A49D913AA6381C01579E3FC00784B49AFA3A771F06389EBC65F8FF3A4E9A7D!  Run `aptos help` for more information about commands

{
  "Result": "Success"
}
```

#### Step 2) Changing the configuration
To change the configuration, you can either run the command `aptos init` or you can manually edit
the `.aptos/config.yaml` that is in your current working directory.

#### Step 3) Creating other profiles

You can also create other profiles for different endpoints and different keys.  These can be made
by adding the `--profile` argument, and can be used in most other commands to replace command line arguments.

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
### Listing resources in an account

You can list the resources in an account from the command line. For example, see below for how to list the resources in the account you just created above:
```bash
$ aptos account list --query resources --account 18B61497FD290B02BB0751F44381CADA1657C2B3AA6194A00D9BC9A85FAD3B04

```

The above command will generate the following resource list information on your terminal:

```bash
{
  "Result": [
    {
      "counter": "2"
    },
    {
      "authentication_key": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
      "self_address": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
      "sequence_number": "0"
    },
    {
      "coin": {
        "value": "10000"
      }
    },
    {
      "received_events": {
        "counter": "0",
        "guid": {
          "guid": {
            "id": {
              "addr": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
              "creation_num": "1"
            }
          },
          "len_bytes": 40
        }
      },
      "sent_events": {
        "counter": "0",
        "guid": {
          "guid": {
            "id": {
              "addr": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
              "creation_num": "0"
            }
          },
          "len_bytes": 40
        }
      }
    }
  ]
}

```

You can additionally list the default profile from configuration with no account specified.
```bash
$ aptos account list
{
  "Result": [
    {
      "counter": "2"
    },
    {
      "authentication_key": "0x50a49d913aa6381c01579e3fc00784b49afa3a771f06389ebc65f8ff3a4e9a7d",
      "self_address": "0x50a49d913aa6381c01579e3fc00784b49afa3a771f06389ebc65f8ff3a4e9a7d",
      "sequence_number": "0"
    },
    {
      "coin": {
        "value": "10000"
      }
    },
    {
      "received_events": {
        "counter": "0",
        "guid": {
          "guid": {
            "id": {
              "addr": "0x50a49d913aa6381c01579e3fc00784b49afa3a771f06389ebc65f8ff3a4e9a7d",
              "creation_num": "1"
            }
          },
          "len_bytes": 40
        }
      },
      "sent_events": {
        "counter": "0",
        "guid": {
          "guid": {
            "id": {
              "addr": "0x50a49d913aa6381c01579e3fc00784b49afa3a771f06389ebc65f8ff3a4e9a7d",
              "creation_num": "0"
            }
          },
          "len_bytes": 40
        }
      }
    }
  ]
}
```

Additionally, any place that takes an account can use the name of a profile:
```bash
$ ./aptos account list --query resources --account superuser
{
  "Result": [
    {
      "counter": "2"
    },
    {
      "authentication_key": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
      "self_address": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
      "sequence_number": "0"
    },
    {
      "coin": {
        "value": "10000"
      }
    },
    {
      "received_events": {
        "counter": "0",
        "guid": {
          "guid": {
            "id": {
              "addr": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
              "creation_num": "1"
            }
          },
          "len_bytes": 40
        }
      },
      "sent_events": {
        "counter": "0",
        "guid": {
          "guid": {
            "id": {
              "addr": "0x18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04",
              "creation_num": "0"
            }
          },
          "len_bytes": 40
        }
      }
    }
  ]
}
```

### Listing modules in an account

You can pass different types of queries to view different items under an account. Currently, 'resources' and
'modules' are supported but more query types are coming. For example, to fetch modules:
```bash
$ ./aptos account list --query modules --account superuser

{
  "Result": [
    {
      "bytecode": "0xa11ceb0b050000000b01000a020a12031c2504410405452d0772e00108d202400692030a0a9c03150cb103650d96040400000101010201030104000506000006080001070700030e0401060100080001000009020300020f0404000110060100041107000003120709010603130a030106050806080105010802020c0a02000103040508020802070801010a0201060c010800010b0301090002070b030109000900074d657373616765054153434949064572726f7273054576656e74065369676e6572124d6573736167654368616e67654576656e740d4d657373616765486f6c64657206537472696e670b6765745f6d6573736167650b7365745f6d6573736167650c66726f6d5f6d6573736167650a746f5f6d657373616765076d657373616765156d6573736167655f6368616e67655f6576656e74730b4576656e7448616e646c650d6e6f745f7075626c697368656406737472696e670a616464726573735f6f66106e65775f6576656e745f68616e646c650a656d69745f6576656e747bd2d264eec4088a11c41a7acbcd8ab2d2c887fa4ea1a3ab0d0b4a405ddfb1560000000000000000000000000000000000000000000000000000000000000001030800000000000000000002020a08020b08020102020c08020d0b030108000001000101030b0a002901030607001102270b002b0110001402010200010105240b0111030c040e0011040c020a02290120030b05120e000b040e00380012012d0105230b022a010c050a051000140c030a050f010b030a04120038010b040b050f0015020100010100",
      "abi": {
        "address": "0x7bd2d264eec4088a11c41a7acbcd8ab2d2c887fa4ea1a3ab0d0b4a405ddfb156",
        "name": "Message",
        "friends": [],
        "exposed_functions": [
          {
            "name": "get_message",
            "visibility": "public",
            "generic_type_params": [],
            "params": [
              "address"
            ],
            "return": [
              "0x1::ASCII::String"
            ]
          },
          {
            "name": "set_message",
            "visibility": "script",
            "generic_type_params": [],
            "params": [
              "signer",
              "vector"
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
                "type": "0x1::ASCII::String"
              },
              {
                "name": "to_message",
                "type": "0x1::ASCII::String"
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
                "type": "0x1::ASCII::String"
              },
              {
                "name": "message_change_events",
                "type": "0x1::Event::EventHandle<0x7bd2d264eec4088a11c41a7acbcd8ab2d2c887fa4ea1a3ab0d0b4a405ddfb156::Message::MessageChangeEvent>"
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
$ ./aptos account transfer --account superuser --amount 100
{
  "Result": {
    "gas_used": 86,
    "balance_changes": {
      "18b61497fd290b02bb0751f44381cada1657c2b3aa6194a00d9bc9a85fad3b04": {
        "coin": {
          "value": "10100"
        }
      },
      "50a49d913aa6381c01579e3fc00784b49afa3a771f06389ebc65f8ff3a4e9a7d": {
        "coin": {
          "value": "9814"
        }
      }
    },
    "sender": "50a49d913aa6381c01579e3fc00784b49afa3a771f06389ebc65f8ff3a4e9a7d",
    "success": true,
    "version": 270408,
    "vm_status": "Executed successfully"
  }
}
```

### Generating a Peer config

To allow others to connect to your node, you need to generate a peer configuration. Below command shows how you can use
the `aptos` CLI to generate a peer configuration and write it into a file named `peer_config.yaml`.
```bash
$ aptos key extract-peer --output-file peer_config.yaml
```

The above command will generate the following output on the terminal:
```bash
{
  "Result": {
    "027eeddfbda3780b51e44731f0b214e53715cd17cdaecac99dc61590c1f2b76a": {
      "addresses": [],
      "keys": [
        "0x027eeddfbda3780b51e44731f0b214e53715cd17cdaecac99dc61590c1f2b76a"
      ],
      "role": "Upstream"
    }
  }
}

```

The `peer_config.yaml` file will be created in your current working directory, with the contents as shown in the below example:
```bash
---
027eeddfbda3780b51e44731f0b214e53715cd17cdaecac99dc61590c1f2b76a:
  addresses: []
  keys:
    - "0x027eeddfbda3780b51e44731f0b214e53715cd17cdaecac99dc61590c1f2b76a"
  role: Upstream
```

**Note:** In the addresses key, you should fill in your address.

### Compiling Move

The `aptos` CLI can be used to compile a Move package locally.
The below example uses the `HelloBlockchain` in [move-examples](../../aptos-move/move-examples/).

```bash
aptos move compile --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71
```

The above command will generate the below terminal output:
```bash
{
  "Result": [
    "8946741E5C907C43C9E042B3739993F32904723F8E2D1491564D38959B59AC71::Message"
  ]
}
```

### Compiling & Unit Testing Move

The `aptos` CLI can also be used to compile and run unit tests locally.
In this example, we'll use the `HelloBlockchain` in [move-examples](../../aptos-move/move-examples/).

```bash
aptos move test --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71
```
The above command will generate the following terminal output:
```bash
BUILDING MoveStdlib
BUILDING AptosFramework
BUILDING Examples
Running Move unit tests
[ PASS    ] 0x8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71::Message::sender_can_set_message
[ PASS    ] 0x8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71::MessageTests::sender_can_set_message
Test result: OK. Total tests: 2; passed: 2; failed: 0
{
  "Result": "Success"
}
```

### Debug and Print Stacktrace

In this example, we will use `DebugDemo` in [debug-move](./debug-move-example)

First, you need to include Move nursery in your Move.toml file [toml file](debug-move-example/Move.toml)

Now, you can use `Debug::print` and `Debug::print_stack_trace` in your [move file](debug-move-example/sources/DebugDemo.move)

You can run the following command:
```bash
aptos move test --package-dir crates/aptos/debug-move-example
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


### Publishing a Move Package with a named address

In this example, we'll use the `HelloBlockchain` in [move-examples](../../aptos-move/move-examples/).

Publish the package with your account address set for `HelloBlockchain`.

Here, you need to change 8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71 to your account address.
```bash
aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71
```

You can additionally use named profiles for the addresses.  The first placeholder is `default`
```bash
aptos move publish --package-dir aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=default
```

### Running a Move Function

Now that you've published the function above, you can run it.

Arguments must be given a type with a colon to separate it.  In this example, we want the input to be
parsed as a string, so we put `string:Hello!`.

```bash
aptos move run --function-id 0x8946741e5c907c43c9e042b3739993f32904723f8e2d1491564d38959b59ac71::Message::set_message --args string:hello!
```

Additionally, profiles can replace addresses in the function id.
```bash
aptos move run --function-id default::Message::set_message --args string:hello!
```

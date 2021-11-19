# Shuffle API

All commands are shown like so: `generic_command` | `example_command`

## shuffle new

### Usage:

`shuffle new <project_path>` | `shuffle new ~/Desktop/TestCoin`

Takes in a project_path and creates a new shuffle project at that location with all pre-generated folders which can be used down the line. Also creates ~/.shuffle/Networks.toml which is your networks configuration file.

Note: Feel free to add any network to connect to with the same format as below to the Networks.toml. To properly add a network to the Networks.toml file, you need the network name, json_rpc_url, and dev_api_url. Additionally, you can use a faucet_url if the specified network supports one, but this field is optional.

    [networks.localhost]
    name = 'localhost'
    json_rpc_url = 'http://127.0.0.1:8080/'
    dev_api_url = 'http://127.0.0.1:8080/'

    [networks.sample_network]
    name = 'sample_network'
    json_rpc_url = 'http://sample_network.com/'
    dev_api_url = 'http://dev.sample_network.com'
    faucet_url = 'http://faucet.sample_network.com'


## shuffle node

### Usage:

`shuffle node`

Runs a local node at default endpoint http://127.0.0.1:8080. Also creates ~/.shuffle/nodeconfig directory which contains information on the configuration of the local node.

`shuffle node --genesis <move_package_path>` | `shuffle node --genesis diem-move/diem-framework/experimental`

Runs a local node with specific move package as the genesis modules. Tutorial: https://github.com/diem/diem/blob/main/shuffle/cli/tutorials/Genesis.md.

## shuffle account

### Usage:

`shuffle account`

Creates an account on onchain on localhost network. Also saves account information locally in the user's ~/.shuffle/networks/localhost/accounts.

`shuffle account --root <mint_key_path>` | `shuffle account --root ~/.shuffle/nodeconfig/mint.key`

Creates an account from root/mint.key path.

`shuffle account --network <network_name>` | `shuffle account --network trove_testnet`

Creates an account on the specified network. Saves account information locally in the user's ~/.shuffle/networks/[network_name]/accounts.

Note: the network name that is passed in must exist in the Networks.toml file. If the network supports a faucet_url, make sure to add that to the Networks.toml. If this field isn't added, the account will be created on localhost instead of the desired network.

## shuffle deploy

### Usage:

`shuffle deploy --project-path <project_path>` | `shuffle deploy --project-path ~/Desktop/TestCoin`

Publishes the main move package in the user's project folder on localhost using the account as publisher.

`shuffle deploy --project-path <project_path> --network <network_name>` |

`shuffle deploy --project-path ~/Desktop/TestCoin --network trove_testnet`

Publishes the main move package on a specified network. Note: the network name that is passed in must exist in the Networks.toml file.

## shuffle console

### Usage:

`shuffle console --project-path <project_path>` | `shuffle console --project-path ~/Desktop/TestCoin`

Enters typescript REPL for onchain inspection of deployed project.

`shuffle console --project-path <project_path> --network <network_name>` |

`shuffle console --project-path ~/Desktop/TestCoin --network trove_testnet`

Enters REPL for onchain inspection on specified network. Note: the network name that is passed in must exist in the Networks.toml file.

`shuffle console --project-path <project_path> --key-path <private_key_path> --address <account_address>` |

`shuffle console --project-path ~/Desktop/TestCoin --key-path ~/.shuffle/networks/localhost/accounts/latest/dev.key --address 0x24163AFCC6E33B0A9473852E18327FA9`

Enters repl for inspection on certain key_path and address. Note: when using the key_path and address flags, they both must be passed in.

## shuffle build

### Usage:

`shuffle build --project-path <project_path>` | `shuffle build --project-path ~/Desktop/TestCoin`

Compiles the move package in the user's project folder and generates typescript files.

## shuffle test

### Usage:

`shuffle test e2e --project-path <project_path>` | `shuffle test e2e --project-path ~/Desktop/TestCoin`

Runs end-to-end test in /project_path/e2e folder on localhost

`shuffle test e2e --project-path <project_path> --network <network_name>` |

`shuffle test e2e --project-path ~/Desktop/TestCoin --network localhost`

Runs end-to-end test in /project_path/e2e folder on specific network.
Note: the network name that is passed in must exist in the Networks.toml file.

`shuffle test unit --project-path <project_path>` |

`shuffle test unit --project-path ~/Desktop/TestCoin`

Runs move unit tests created by the user in the move files in /project_path/main/sources

`shuffle test all --project-path <project_path> --network <network_name>` |

`shuffle test all --project-path ~/Desktop/TestCoin --network trove_testnet`

Runs both move unit tests in /project_path/main/sources and end-to-end test in /project_path/e2e on specific network.
Note: the network name that is passed in must exist in the Networks.toml file.

## shuffle transactions

### Usage:

`shuffle transactions`

Displays last 10 transactions from the account on the localhost network in pretty formatting.

`shuffle transactions --raw`

Displays last 10 transactions from the account on the localhost network without pretty formatting.

`shuffle transactions -t`

Displays last 10 transactions from the account on the localhost network in pretty formatting and blocks/continuously polls for incoming transactions.

`shuffle transactions --network <network_name>` | `shuffle transactions --network localhost`

Displays the last 10 transactions from a given network in pretty formatting.
Note: the network name that is passed in must exist in the Networks.toml file.

`shuffle transactions --address <account_address>` | `shuffle transactions --address 24163AFCC6E33B0A9473852E18327FA9`

Displays the last 10 transactions deployed by a given address.

### These flags can be used together in a number of ways:

`shuffle transactions --network trove_testnet --address 0x0000000000000000000000000B1E55ED -t --raw`

Displays the last 10 transactions of address 0xB1E55ED on network trove_testnet without pretty formatting and also blocks and continuously polls for incoming transactions

/// ---
/// title: "Integrate with Velor"
/// slug: "system-integrators-guide"
/// ---
///
/// import Tabs from '@theme/Tabs';
/// import TabItem from '@theme/TabItem';
///
/// # Integrate with the Velor Blockchain
///
/// If you provide blockchain services to your customers and wish to add the Velor blockchain to your platform, then this guide is for you. This system integrators guide will walk you through all you need to integrate the Velor blockchain into your platform.
///
/// ## Overview
///
/// This document will guide you through the following tasks to integrate with Velor:
/// 1. Prepare an environment for testing.
/// 1. Create an account on the blockchain.
/// 1. Exchange account identifiers with another entity on the blockchain, for example, to perform swaps.
/// 1. Create a transaction.
/// 1. Obtain a gas estimate and validate the transaction for correctness.
/// 1. Submit the transaction to the blockchain.
/// 1. Wait for the outcome of the transaction.
/// 1. Query historical transactions and interactions for a given account with a specific account, i.e., withdraws and deposits.
///
/// ## Getting Started
///
/// In order to get started you'll need to select a network and pick your set of tools. There are also a handful of SDKs to help accelerate development.
///
/// ### Choose a network
///
/// There are four well-supported networks for integrating with the Velor blockchain:
///
/// 1. [Localnet](http://127.0.0.1:8080) -- our standalone tool for local development against a known version of the codebase with no external network.
/// 1. [Devnet](https://fullnode.devnet.velorlabs.com/v1/spec#/) -- a shared resource for the community, data resets weekly, weekly update from velor-core main branch.
/// 1. [Testnet](https://fullnode.testnet.velorlabs.com/v1/spec#/) -- a shared resource for the community, data will be preserved, network configuration will mimic Mainnet.
/// 1. [Mainnet](https://fullnode.mainnet.velorlabs.com/v1/spec#/) -- a production network with real assets.
///
/// See [Velor Blockchain Networks](../nodes/networks.md) for full details on each environment.
///
/// ### Run a localnet
///
/// There are two options for running a localnet:
/// * Directly [run a localnet](../nodes/local-testnet/run-a-local-testnet.md) using either the [Velor-core source code](../nodes/local-testnet/run-a-local-testnet.md#using-the-velor-core-source-code) or a [Docker image](../nodes/local-testnet/run-a-local-testnet.md#using-docker). These paths are useful for testing changes to the Velor-core codebase or framework, or for building services on top of the Velor blockchain, respectively.
/// * [Install the Velor CLI](../tools/install-cli/index.md) and 2) start a [local node with a faucet](../nodes/local-testnet/using-cli-to-run-a-local-testnet.md#starting-a-local-testnet-with-a-faucet). This path is useful for developing on the Velor blockchain, debugging Move contracts, and testing node operations.
///
/// Either of these methods will expose a [REST API service](../integration/fullnode-rest-api.md) at `http://127.0.0.1:8080` and a Faucet API service at `http://127.0.0.1:8000` for option 1 run a localnet or `http://127.0.0.1:8081` for option 2 install the Velor CLI. The applications will output the location of the services.
///
/// ### Production network access
///
/// <Tabs groupId="networks">
///   <TabItem value="devnet" label="Devnet">
///     <ul>
///       <li>REST API: <a href="https://fullnode.devnet.velorlabs.com/v1">https://fullnode.devnet.velorlabs.com/v1</a></li>
///       <li>REST API Spec: <a href="https://fullnode.devnet.velorlabs.com/v1/spec#/">https://fullnode.devnet.velorlabs.com/v1/spec#/</a></li>
///       <li>Indexer API: <a href="https://api.devnet.velorlabs.com/v1/graphql">https://api.devnet.velorlabs.com/v1/graphql</a></li>
///       <li>Faucet API: <a href="https://faucet.devnet.velorlabs.com">https://faucet.devnet.velorlabs.com</a></li>
///       <li><a href="https://cloud.hasura.io/public/graphiql?endpoint=https://api.devnet.velorlabs.com/v1/graphql">Indexer GraphQL</a></li>
///     </ul>
///   </TabItem>
///   <TabItem value="testnet" label="Testnet">
///     <ul>
///       <li>REST API: <a href="https://fullnode.testnet.velorlabs.com/v1">https://fullnode.testnet.velorlabs.com/v1</a></li>
///       <li>REST API Spec: <a href="https://fullnode.testnet.velorlabs.com/v1/spec#/">https://fullnode.testnet.velorlabs.com/v1/spec#/</a></li>
///       <li>Indexer API: <a href="https://api.testnet.velorlabs.com/v1/graphql">https://api.testnet.velorlabs.com/v1/graphql</a></li>
///       <li>Faucet API: <a href="https://faucet.testnet.velorlabs.com">https://faucet.testnet.velorlabs.com</a></li>
///       <li><a href="https://cloud.hasura.io/public/graphiql?endpoint=https://api.testnet.velorlabs.com/v1/graphql">Indexer GraphQL</a></li>
///     </ul>
///   </TabItem>
///   <TabItem value="mainnet" label="Mainnet">
///     <ul>
///       <li>REST API: <a href="https://fullnode.mainnet.velorlabs.com/v1">https://fullnode.mainnet.velorlabs.com/v1</a></li>
///       <li>REST API Spec: <a href="https://fullnode.mainnet.velorlabs.com/v1/spec#/">https://fullnode.mainnet.velorlabs.com/v1/spec#/</a></li>
///       <li>Indexer API: <a href="https://api.mainnet.velorlabs.com/v1/graphql">https://api.mainnet.velorlabs.com/v1/graphql</a></li>
///       <li>Faucet: N/A</li>
///       <li><a href="https://cloud.hasura.io/public/graphiql?endpoint=https://api.mainnet.velorlabs.com/v1/graphql">Indexer GraphQL</a></li>
///     </ul>
///   </TabItem>
/// </Tabs>
///
/// ### SDKs and tools
///
/// Velor currently provides three SDKs:
/// 1. [Typescript](../sdks/ts-sdk/index.md)
/// 2. [Python](../sdks/python-sdk.md)
/// 3. [Rust](../sdks/rust-sdk.md)
///
/// Almost all developers will benefit from exploring the CLI. [Using the CLI](../tools/velor-cli-tool/use-velor-cli.md) demonstrates how the CLI can be used to which includes creating accounts, transferring coins, and publishing modules.
///
/// ## Accounts on Velor
///
///
/// At creation, an [Velor account](https://github.com/velor-chain/velor-core/blob/88c9aab3982c246f8aa75eb2caf8c8ab1dcab491/velor-move/framework/velor-framework/sources/account.move#L23) contains:
/// * A [resource containing Velor Coin](https://github.com/velor-chain/velor-core/blob/60751b5ed44984178c7163933da3d1b18ad80388/velor-move/framework/velor-framework/sources/coin.move#L50) and deposit and withdrawal of coins from that resource.
/// * An authentication key associated with their current public, private key(s).
/// * A strictly increasing [sequence number](../concepts/accounts.md#account-sequence-number) that represents the account's next transaction's sequence number to prevent replay attacks.
/// * A strictly increasing number that represents the next distinct GUID creation number.
/// * An [event handle](../concepts/events.md) for all new types of coins added to the account.
/// * An event handle for all key rotations for the account.
///
/// Read more about [Accounts](../concepts/accounts.md) and [set one up](../tools/velor-cli-tool/use-velor-cli#initialize-local-configuration-and-create-an-account).
///
/// ## Transactions
///
///
/// Read more in [Transactions and States](../concepts/txns-states.md).
///
/// ### Generating transactions
///
/// Velor supports two methods for constructing transactions:
///
/// - Using the Velor client libraries to generate native BCS transactions.
/// - Constructing JSON-encoded objects and interacting with the REST API to generate native transactions.
///
/// The preferred approach is to directly generate native BCS transactions. Generating them via the REST API enables rapid development at the cost of trusting the fullnode to generate the transaction correctly.
///
/// #### BCS-encoded transactions
///
///
/// #### JSON-encoded transactions
///
///
///
///
/// ### Types of transactions
///
/// Within a given transaction, the target of execution can be one of two types:
///
/// - An entry point (formerly known as script function)
/// - A script (payload)
///
/// Both [Python](https://github.com/velor-chain/velor-core/blob/3973311dac6bb9348bfc81cf983c2a1be11f1b48/ecosystem/python/sdk/velor_sdk/client.py#L256) and [TypeScript](https://github.com/velor-chain/velor-core/blob/3973311dac6bb9348bfc81cf983c2a1be11f1b48/ecosystem/typescript/sdk/src/velor_client.test.ts#L93) support the generation of transactions that target entry points. This guide points out many of those entry points, such as `velor_account::transfer` and `velor_account::create_account`.
///
/// Most basic operations on the Velor blockchain should be available via entry point calls. While one could submit multiple transactions calling entry points in series, such operations benefit from being called atomically from a single transaction. A script payload transaction can call any public (entry) function defined within any module. Here's an example [Move script](https://github.com/velor-chain/velor-core/tree/main/velor-move/move-examples/scripts/two_by_two_transfer) that uses a MultiAgent transaction to extract funds from two accounts and deposit them into two other accounts. This is a [Python example](https://github.com/velor-chain/velor-core/blob/main/ecosystem/python/sdk/examples/transfer-two-by-two.py) that uses the bytecode generated by compiling that script. Currently there is limited support for script payloads in TypeScript.
///
/// ### Status of a transaction
///
///
/// A reasonable strategy for submitting transactions is to limit their lifetime to 30 to 60 seconds, and polling that API at regular intervals until success or several seconds after that time has elapsed. If there is no commitment on-chain, the transaction was likely discarded.
///
/// ### Testing transactions or transaction pre-execution
///
/// To facilitate evaluation of transactions as well as gas estimation, Velor supports a simulation API that does not require and should not contain valid signatures on transactions.
///
///
/// Both the [Typescript SDK](https://github.com/velor-chain/velor-core/blob/9b85d41ed8ef4a61a9cd64f9de511654fcc02024/ecosystem/typescript/sdk/src/velor_client.ts#L413) and [Python SDK](https://github.com/velor-chain/velor-core/blob/main/ecosystem/python/sdk/examples/simulate-transfer-coin.py) support the simulation API. Note the output and gas used may change based upon the state of the account. For gas estimations, we recommend that the maximum gas amount be larger than the amount quoted by this API.
///
/// ## Viewing current and historical state
///
/// Most integrations into the Velor blockchain benefit from a holistic and comprehensive overview of the current and historical state of the blockchain. Velor provides historical transactions, state, and events, all the result of transaction execution.
///
/// * Historical transactions specify the execution status, output, and tie to related events. Each transaction has a unique version number associated with it that dictates its global sequential ordering in the history of the blockchain ledger.
/// * The state is the representation of all transaction outputs up to a specific version. In other words, a state version is the accumulation of all transactions inclusive of that transaction version.
/// * As transactions execute, they may emit events. [Events](../concepts/events.md) are hints about changes in on-chain data.
///
/// The storage service on a node employs two forms of pruning that erase data from nodes:
///
/// * state
/// * events, transactions, and everything else
///
/// While either of these may be disabled, storing the state versions is not particularly sustainable.
///
/// Events and transactions pruning can be disabled via setting the [`enable_ledger_pruner`](https://github.com/velor-chain/velor-core/blob/cf0bc2e4031a843cdc0c04e70b3f7cd92666afcf/config/src/config/storage_config.rs#L141) to `false`. This is default behavior in Mainnet. In the near future, Velor will provide indexers that mitigate the need to directly query from a node.
///
/// The REST API offers querying transactions and events in these ways:
///
/// * [Transactions for an account](https://fullnode.devnet.velorlabs.com/v1/spec#/operations/get_account_transactions)
/// * [Transactions by version](https://fullnode.devnet.velorlabs.com/v1/spec#/operations/get_transaction_by_version)
/// * [Events by event handle](https://fullnode.devnet.velorlabs.com/v1/spec#/operations/get_events_by_event_handle)
///
/// ## Exchanging and tracking coins
///
/// Velor has a standard [Coin type](https://github.com/velor-chain/velor-core/blob/main/velor-move/framework/velor-framework/sources/coin.move). Different types of coins can be represented in this type through the use of distinct structs that represent the type parameter or generic for `Coin<T>`.
///
/// Coins are stored within an account under the resource `CoinStore<T>`. At account creation, each user has the resource `CoinStore<0x1::velor_coin::VelorCoin>` or `CoinStore<VelorCoin>`, for short. Within this resource is the Velor coin: `Coin<VelorCoin>`.
///
/// ### Transferring coins between users
///
/// Coins can be transferred between users via the [`coin::transfer`](https://github.com/velor-chain/velor-core/blob/36a7c00b29a457469264187d8e44070b2d5391fe/velor-move/framework/velor-framework/sources/coin.move#L307) function for all coins and [`velor_account::transfer`](https://github.com/velor-chain/velor-core/blob/88c9aab3982c246f8aa75eb2caf8c8ab1dcab491/velor-move/framework/velor-framework/sources/velor_account.move#L18) for Velor coins. The advantage of the latter function is that it creates the destination account if it does not exist.
///
/// :::caution
/// It is important to note that if an account has not registered a `CoinStore<T>` for a given `T`, then any transfer of type `T` to that account will fail.
/// :::
///
/// ### Current balance for a coin
///
/// The current balance for a `Coin<T>` where `T` is the Velor coin is available at the account resources URL: `https://{rest_api_server}/accounts/{address}/resource/0x1::coin::CoinStore<0x1::velor_coin::VelorCoin>`. The balance is stored within `coin::amount`. The resource also contains the total number of deposit and withdraw events, and the `counter` value within `deposit_events` and `withdraw_events`, respectively.
///
/// ```
/// {
///   "type": "0x1::coin::CoinStore<0x1::velor_coin::VelorCoin>",
///   "data": {
///     "coin": {
///       "value": "3927"
///     },
///     "deposit_events": {
///       "counter": "1",
///       "guid": {
///         "id": {
///           "addr": "0xcb2f940705c44ba110cd3b4f6540c96f2634938bd5f2aabd6946abf12ed88457",
///           "creation_num": "2"
///         }
///       }
///     },
///     "withdraw_events": {
///       "counter": "1",
///       "guid": {
///         "id": {
///           "addr": "0xcb2f940705c44ba110cd3b4f6540c96f2634938bd5f2aabd6946abf12ed88457",
///           "creation_num": "3"
///         }
///       }
///     }
///   }
/// }
/// ```
///
/// ### Querying transactions
///
/// In Velor, each transaction is committed as a distinct version to the blockchain. This allows for the convenience of sharing committed transactions by their version number; to do so, query: `https://{rest_server_api}/transactions/by_version/{version}`
///
/// Transactions submitted by an account can also be queried via the following URL where the `sequence_number` matches the sequence number of the transaction: `https://{rest_server_api}/account/{address}/transactions?start={sequence_number}&limit=1`
///
/// A transfer transaction would appear as follows:
///
/// ```
/// {
///   "version": "13629679",
///   "gas_used": "4",
///   "success": true,
///   "vm_status": "Executed successfully",
///   "changes": [
///     {
///       "address": "0xb258b91eee04111039320a85b0c24a2dd433909e14a6b5c32ee722e0fdecfddc",
///       "data": {
///         "type": "0x1::coin::CoinStore<0x1::velor_coin::VelorCoin>",
///         "data": {
///           "coin": {
///             "value": "1000"
///           },
///           "deposit_events": {
///             "counter": "1",
///             "guid": {
///               "id": {
///                 "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///                 "creaton_num": "2",
///               }
///             }
///           },
///           ...
///         }
///       },
///       "type": "write_resource"
///     },
///     ...
///   ],
///   "sender": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
///   "sequence_number": "0",
///   "max_gas_amount": "2000",
///   "gas_unit_price": "1",
///   "expiration_timestamp_secs": "1660616127",
///   "payload": {
///     "function": "0x1::coin::transfer",
///     "type_arguments": [
///       "0x1::velor_coin::VelorCoin"
///     ],
///     "arguments": [
///       "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///       "1000"
///     ],
///     "type": "entry_function_payload"
///   },
///   "events": [
///     {
///       "key": "0x0300000000000000810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
///       "guid": {
///         "id": {
///           "addr": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
///           "creation_num": "3"
///           }
///         }
///       },
///       "sequence_number": "0",
///       "type": "0x1::coin::WithdrawEvent",
///       "data": {
///         "amount": "1000"
///       }
///     },
///     {
///       "key": "0x02000000000000005098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///       guid": {
///         "id": {
///           "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///           "creation_num": "2"
///           }
///         }
///       },
///       "sequence_number": "0",
///       "type": "0x1::coin::DepositEvent",
///       "data": {
///         "amount": "1000"
///       }
///     }
///   ],
///   "timestamp": "1660615531147935",
///   "type": "user_transaction"
/// }
///
/// ```
///
/// Here is a breakdown of the information in a transaction:
/// * `version` indicates the globally unique identifier for this transaction, its ordered position in all the committed transactions on the blockchain
/// * `sender` is the account address of the entity that submitted the transaction
/// * `gas_used` is the units paid for executing the transaction
/// * `success` and `vm_status` indicate whether or not the transaction successfully executed and any reasons why it might not have
/// * `changes` include the final values for any state resources that have been modified during the execution of the transaction
/// * `events` contain all the events emitted during the transaction execution
/// * `timetstamp` is the near real-time timestamp of the transaction's execution
///
/// If `success` is false, then `vm_status` will contain an error code or message that resulted in the transaction failing to succeed. When `success` is false, `changes` will be limited to gas deducted from the account and the sequence number incrementing. There will be no `events`.
///
/// Each event in `events` is differentiated by a `key`. The `key` is derived from the `guid` in `changes`. Specifically, the `key` is a 40-byte hex string where the first eight bytes (or 16 characters) are the little endian representation of the `creation_num` in the `guid` of the `changes` event, and the remaining characters are the account address.
///
/// As events do not dictate what emitted them, it is imperative to track the path in `changes` to determine the source of an event. In particular, each `CoinStore<T>` has both a `WithdrawEvent` and a `DepositEvent`, based upon the type of coin. In order to determine which coin type is used in a transaction, an indexer can compare the `guid::creation_num` in a `changes` event combined with the address to the `key` for events in `events`.
///
/// Using the above example, `events[1].guid` is equivalent to `changes[0].data.data.deposit_events.guid`, which is `{"addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e", "creation_num": "2"}`.
///
/// :::tip
/// The `key` field will be going away in favor of `guid`
/// :::
///
/// ### Querying events
///
/// Velor provides clear and canonical events for all withdraw and deposit of coins. This can be used in coordination with the associated transactions to present to a user the change of their account balance over time, when that happened, and what caused it. With some amount of additional parsing, metadata such as the transaction type and the other parties involved can also be shared.
///
/// Query events by handle URL: `https://{rest_api_server}/accounts/{address}/events/0x1::coin::CoinStore<0x1::velor_coin::VelorCoin>/withdraw_events`
///
/// ```
/// [
///   {
///     "version":"13629679",
///     "key": "0x0300000000000000cb2f940705c44ba110cd3b4f6540c96f2634938bd5f2aabd6946abf12ed88457",
///     "guid": {
///       "id": {
///         "addr": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
///         "creation_num": "3"
///         }
///       }
///     },
///     "sequence_number": "0",
///     "type": "0x1::coin::WithdrawEvent",
///     "data": {
///       "amount": "1000"
///     }
///   }
/// ]
/// ```
///
/// Gather more information from the transaction that generated the event by querying `https://{rest_server_api}/transactions/by_version/{version}` where `{version}` is the same value as the `{version}` in the event query.
///
/// :::tip
///
/// When tracking full movement of coins, normally events are sufficient. `0x1::velor_coin::VelorCoin`, however, requires considering `gas_used` for each transaction sent from the given account since it represents gas in Velor. To reduce unnecessary overhead, extracting gas fees due to transactions does not emit an event. All transactions for an account can be retrieved from this API: `https://{rest_server_api}/accounts/{address}/transactions`
///
/// :::
///
/// ### Tracking coin balance changes
///
/// Consider the transaction from the earlier section, but now with an arbitrary coin `0x1337::my_coin::MyCoin` and some gas parameters changed:
/// ```
/// {
///   "version": "13629679",
///   "gas_used": "20",
///   "success": true,
///   "vm_status": "Executed successfully",
///   "changes": [
///     {
///       "address": "0xb258b91eee04111039320a85b0c24a2dd433909e14a6b5c32ee722e0fdecfddc",
///       "data": {
///         "type": "0x1::coin::CoinStore<0x1337::my_coin::MyCoin>",
///         "data": {
///           "coin": {
///             "value": "1000"
///           },
///           "deposit_events": {
///             "counter": "1",
///             "guid": {
///               "id": {
///                 "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///                 "creaton_num": "2",
///               }
///             }
///           },
///           ...
///         }
///       },
///       "type": "write_resource"
///     },
///     ...
///   ],
///   "sender": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
///   "sequence_number": "0",
///   "max_gas_amount": "2000",
///   "gas_unit_price": "110",
///   "expiration_timestamp_secs": "1660616127",
///   "payload": {
///     "function": "0x1::coin::transfer",
///     "type_arguments": [
///       "0x1337::my_coin::MyCoin"
///     ],
///     "arguments": [
///       "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///       "1000"
///     ],
///     "type": "entry_function_payload"
///   },
///   "events": [
///     {
///       "key": "0x0300000000000000810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
///       "guid": {
///         "id": {
///           "addr": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
///           "creation_num": "3"
///           }
///         }
///       },
///       "sequence_number": "0",
///       "type": "0x1::coin::WithdrawEvent",
///       "data": {
///         "amount": "1000"
///       }
///     },
///     {
///       "key": "0x02000000000000005098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///       guid": {
///         "id": {
///           "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
///           "creation_num": "2"
///           }
///         }
///       },
///       "sequence_number": "0",
///       "type": "0x1::coin::DepositEvent",
///       "data": {
///         "amount": "1000"
///       }
///     }
///   ],
///   "timestamp": "1660615531147935",
///   "type": "user_transaction"
/// }
/// ```
///
/// There are three balance changes in this transaction:
/// 1. A withdrawal of `1000` of `0x1337::my_coin::MyCoin` from the transaction sending account `0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b`
/// 2. A deposit of `1000` of `0x1337::my_coin::MyCoin` to receiving account `0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e`
/// 3. A gas fee `2200` of `0x1::velor_coin::VelorCoin` from the sending account `0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b`
///
/// To retrieve the withdrawal information:
/// 1. Scan the `changes` for `0x1::coin::CoinStore<CoinType>`.  Note the `CoinType` is a generic signifying which coin is stored in the store.  In this example, the `CoinType` is `0x1337::my_coin::MyCoin`.
/// 2. Retrieve the `guid` for `withdraw_events`. In this example, the `guid` contains `addr` `0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b` and `creation_num` `3`.
/// 3. Scan for events with this `guid` and extract the event associated with it.  In this example, it is the `0x1::coin::WithdrawEvent`.
/// 4. Note the `amount` field will be the number of `CoinType` removed from the account in the `guid`. In this example, it is `1000`.
///
/// To retrieve the deposit information, it's the same as withdrawal except:
/// 1. The `guid` used is under `deposit_events`
/// 2. The `amount` will be a positive increase on the account's balance.
/// 3. The event's name will be: `0x1::coin::DepositEvent`
///
/// To retrieve the gas fee:
/// 1. The `gas_used` field must be multiplied times the `gas_unit_price`.  In this example, `gas_used=20` and `gas_unit_price=110` so the total gas coins withdrawn is `2200`.
/// 2. Gas is always: `0x1::velor_coin::VelorCoin`
///
/// To retrieve information about the number of decimals of the coin:
/// 1. You can retrieve the number of decimals for a coin via its: `0x1::coin::CoinInfo<CoinType>`
/// 2. This will be located at the address of the coin type.  In this example, you would need to look up `0x1::coin::CoinInfo<0x1337::my_coin::MyCoin>` at address `0x1337`.
///
/// :::tip
/// If you always use the events in this manner, you won't miss any balance changes for an account.
/// By monitoring the events, you will find all balance changes in the `0x1::coin::CoinStore`:
/// 1. Coin mints
/// 2. Coin burns
/// 3. Coin transfers
/// 4. Staking coins
/// 5. Withdrawing staked coins
/// 6. Transfers not derived from `coin::transfer`
///
/// :::
///
/// To create some sample data to explore, conduct ["Your first transaction"](../tutorials/first-transaction.md).
///
/// To learn more about coin creation, make ["Your First Coin"](../tutorials/first-coin.md).
///
/// ## Integrating with the faucet
///
/// This tutorial is for SDK and wallet developers who want to integrate with the [Velor Faucet](https://github.com/velor-chain/velor-core/tree/main/crates/velor-faucet). If you are a dapp developer, you should access the faucet through an existing [SDK](../tutorials/first-transaction.md) or [CLI](../tools/velor-cli-tool/use-velor-cli#initialize-local-configuration-and-create-an-account) instead.
///
/// ### Differences between devnet and testnet
/// What are the differences between devnet and testnet? Effectively none. In the past, the testnet faucet had a Captcha in front of it, making it unqueryable by normal means. This is no longer true.
///
/// The endpoints for each faucet are:
/// - Devnet: https://faucet.devnet.velorlabs.com
/// - Testnet: https://faucet.testnet.velorlabs.com
///
/// ### Calling the faucet: JavaScript / TypeScript
/// If you are building a client in JavaScript or TypeScript, you should make use of the [@velor-chain/velor-faucet-client](https://www.npmjs.com/package/@velor-chain/velor-faucet-client) package. This client is generated based on the OpenAPI spec published by the faucet service.
///
/// Example use:
/// ```typescript
/// import {
///   VelorFaucetClient,
///   FundRequest,
/// } from "@velor-chain/velor-faucet-client";
///
/// async function callFaucet(amount: number, address: string): Promise<string[]> {
///   const faucetClient = new VelorFaucetClient({BASE: "https://faucet.devnet.velorlabs.com"});
///   const request: FundRequest = {
///     amount,
///     address,
///   };
///   const response = await faucetClient.fund({ requestBody: request });
///   return response.txn_hashes;
/// }
/// ```
///
/// ### Calling the faucet: Other languages
/// If you are trying to call the faucet in other languages, you have two options:
/// 1. Generate a client from the [OpenAPI spec](https://github.com/velor-chain/velor-core/blob/main/crates/velor-faucet/doc/spec.yaml).
/// 2. Call the faucet on your own.
///
/// For the latter, you will want to build a query similar to this:
/// ```
/// curl -X POST 'https://faucet.devnet.velorlabs.com/mint?amount=10000&address=0xd0f523c9e73e6f3d68c16ae883a9febc616e484c4998a72d8899a1009e5a89d6'
/// ```
///
/// This means mint 10000 OCTA to address `0xd0f523c9e73e6f3d68c16ae883a9febc616e484c4998a72d8899a1009e5a89d6`.
module large_package_example::five {
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    /// Long winded text that goes on and on and on
    public fun large_vector(): vector<u8> {
        x"55555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555"
    }

    public fun another_large_vector(): vector<u8> {
        x"55555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555555"
    }
}

---
title: "Use Aptos API"
slug: "aptos-api"
---

# Use the Aptos REST Read API

If you provide blockchain services to your customers and wish to add the Aptos blockchain to your platform, then this guide is for you. This guide will walk you through all you need to integrate the Aptos blockchain into your platform.

:::tip
Before starting in this guide, it might be beneficial to read the [System Integrators Guide](/guides/system-integrators-guide).
:::

## Viewing current and historical state

Most integrations into the Aptos blockchain benefit from a holistic and comprehensive overview of the current and historical state of the blockchain. Aptos provides historical transactions, state, and events, all the result of transaction execution.

* Historical transactions specify the execution status, output, and tie to related events. Each transaction has a unique version number associated with it that dictates its global sequential ordering in the history of the blockchain ledger.
* The state is the representation of all transaction outputs up to a specific version. In other words, a state version is the accumulation of all transactions inclusive of that transaction version.
* As transactions execute, they may emit events. [Events](/concepts/events) are hints about changes in on-chain data.

The storage service on a node employs two forms of pruning that erase data from nodes: 

* state
* events, transactions, and everything else

While either of these may be disabled, storing the state versions is not particularly sustainable. 

Events and transactions pruning can be disabled via setting the [`enable_ledger_pruner`](https://github.com/aptos-labs/aptos-core/blob/cf0bc2e4031a843cdc0c04e70b3f7cd92666afcf/config/src/config/storage_config.rs#L141) to `false`. This is default behavior in Mainnet. In the near future, Aptos will provide indexers that mitigate the need to directly query from a node.

The REST API offers querying transactions and events in these ways:

* [Transactions for an account](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_account_transactions)
* [Transactions by version](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_transaction_by_version)
* [Events by event handle](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_events_by_event_handle)

## Exchanging and tracking coins

Aptos has a standard [Coin type](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/coin.move). Different types of coins can be represented in this type through the use of distinct structs that represent the type parameter or generic for `Coin<T>`. 

Coins are stored within an account under the resource `CoinStore<T>`. At account creation, each user has the resource `CoinStore<0x1::aptos_coin::AptosCoin>` or `CoinStore<AptosCoin>`, for short. Within this resource is the Aptos coin: `Coin<AptosCoin>`.

### Transferring coins between users

Coins can be transferred between users via the [`coin::transfer`](https://github.com/aptos-labs/aptos-core/blob/36a7c00b29a457469264187d8e44070b2d5391fe/aptos-move/framework/aptos-framework/sources/coin.move#L307) function for all coins and [`aptos_account::transfer`](https://github.com/aptos-labs/aptos-core/blob/88c9aab3982c246f8aa75eb2caf8c8ab1dcab491/aptos-move/framework/aptos-framework/sources/aptos_account.move#L18) for Aptos coins. The advantage of the latter function is that it creates the destination account if it does not exist. 

:::caution
It is important to note that if an account has not registered a `CoinStore<T>` for a given `T`, then any transfer of type `T` to that account will fail.
:::

### Current balance for a coin

The current balance for a `Coin<T>` where `T` is the Aptos coin is available at the account resources URL: `https://{rest_api_server}/accounts/{address}/resource/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>`. The balance is stored within `coin::amount`. The resource also contains the total number of deposit and withdraw events, and the `counter` value within `deposit_events` and `withdraw_events`, respectively.

```
{
  "type": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
  "data": {
    "coin": {
      "value": "3927"
    },
    "deposit_events": {
      "counter": "1",
      "guid": {
        "id": {
          "addr": "0xcb2f940705c44ba110cd3b4f6540c96f2634938bd5f2aabd6946abf12ed88457",
          "creation_num": "2"
        }
      }
    },
    "withdraw_events": {
      "counter": "1",
      "guid": {
        "id": {
          "addr": "0xcb2f940705c44ba110cd3b4f6540c96f2634938bd5f2aabd6946abf12ed88457",
          "creation_num": "3"
        }
      }
    }
  }
}
```

### Querying transactions

In Aptos, each transaction is committed as a distinct version to the blockchain. This allows for the convenience of sharing committed transactions by their version number; to do so, query: `https://{rest_server_api}/transactions/by_version/{version}`

Transactions submitted by an account can also be queried via the following URL where the `sequence_number` matches the sequence number of the transaction: `https://{rest_server_api}/account/{address}/transactions?start={sequence_number}&limit=1`

A transfer transaction would appear as follows:

```
{
  "version": "13629679",
  "gas_used": "4",
  "success": true,
  "vm_status": "Executed successfully",
  "changes": [
    {
      "address": "0xb258b91eee04111039320a85b0c24a2dd433909e14a6b5c32ee722e0fdecfddc",
      "data": {
        "type": "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
        "data": {
          "coin": {
            "value": "1000"
          },
          "deposit_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
                "creaton_num": "2",
              }
            }
          },
          ...
        }
      },
      "type": "write_resource"
    },
    ...
  ],
  "sender": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
  "sequence_number": "0",
  "max_gas_amount": "2000",
  "gas_unit_price": "1",
  "expiration_timestamp_secs": "1660616127",
  "payload": {
    "function": "0x1::coin::transfer",
    "type_arguments": [
      "0x1::aptos_coin::AptosCoin"
    ],
    "arguments": [
      "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
      "1000"
    ],
    "type": "entry_function_payload"
  },
  "events": [
    {
      "key": "0x0300000000000000810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
      "guid": {
        "id": {
          "addr": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
          "creation_num": "3"
          }
        }
      },
      "sequence_number": "0",
      "type": "0x1::coin::WithdrawEvent",
      "data": {
        "amount": "1000"
      }
    },
    {
      "key": "0x02000000000000005098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
      guid": {
        "id": {
          "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
          "creation_num": "2"
          }
        }
      },
      "sequence_number": "0",
      "type": "0x1::coin::DepositEvent",
      "data": {
        "amount": "1000"
      }
    }
  ],
  "timestamp": "1660615531147935",
  "type": "user_transaction"
}

```

Here is a breakdown of the information in a transaction:
* `version` indicates the globally unique identifier for this transaction, its ordered position in all the committed transactions on the blockchain
* `sender` is the account address of the entity that submitted the transaction
* `gas_used` is the units paid for executing the transaction
* `success` and `vm_status` indicate whether or not the transaction successfully executed and any reasons why it might not have
* `changes` include the final values for any state resources that have been modified during the execution of the transaction
* `events` contain all the events emitted during the transaction execution
* `timetstamp` is the near real-time timestamp of the transaction's execution

If `success` is false, then `vm_status` will contain an error code or message that resulted in the transaction failing to succeed. When `success` is false, `changes` will be limited to gas deducted from the account and the sequence number incrementing. There will be no `events`.

Each event in `events` is differentiated by a `key`. The `key` is derived from the `guid` in `changes`. Specifically, the `key` is a 40-byte hex string where the first eight bytes (or 16 characters) are the little endian representation of the `creation_num` in the `guid` of the `changes` event, and the remaining characters are the account address.

As events do not dictate what emitted them, it is imperative to track the path in `changes` to determine the source of an event. In particular, each `CoinStore<T>` has both a `WithdrawEvent` and a `DepositEvent`, based upon the type of coin. In order to determine which coin type is used in a transaction, an indexer can compare the `guid::creation_num` in a `changes` event combined with the address to the `key` for events in `events`.

Using the above example, `events[1].guid` is equivalent to `changes[0].data.data.deposit_events.guid`, which is `{"addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e", "creation_num": "2"}`.

:::tip
The `key` field will be going away in favor of `guid`
:::

### Querying events

Aptos provides clear and canonical events for all withdraw and deposit of coins. This can be used in coordination with the associated transactions to present to a user the change of their account balance over time, when that happened, and what caused it. With some amount of additional parsing, metadata such as the transaction type and the other parties involved can also be shared.

Query events by handle URL: `https://{rest_api_server}/accounts/{address}/events/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>/withdraw_events`

```
[
  {
    "version":"13629679",
    "key": "0x0300000000000000cb2f940705c44ba110cd3b4f6540c96f2634938bd5f2aabd6946abf12ed88457",
    "guid": {
      "id": {
        "addr": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
        "creation_num": "3"
        }
      }
    },
    "sequence_number": "0",
    "type": "0x1::coin::WithdrawEvent",
    "data": {
      "amount": "1000"
    }
  }
]
```

Gather more information from the transaction that generated the event by querying `https://{rest_server_api}/transactions/by_version/{version}` where `{version}` is the same value as the `{version}` in the event query.

:::tip

When tracking full movement of coins, normally events are sufficient. `0x1::aptos_coin::AptosCoin`, however, requires considering `gas_used` for each transaction sent from the given account since it represents gas in Aptos. To reduce unnecessary overhead, extracting gas fees due to transactions does not emit an event. All transactions for an account can be retrieved from this API: `https://{rest_server_api}/accounts/{address}/transactions`

:::

### Tracking coin balance changes

Consider the transaction from the earlier section, but now with an arbitrary coin `0x1337::my_coin::MyCoin` and some gas parameters changed:
```
{
  "version": "13629679",
  "gas_used": "20",
  "success": true,
  "vm_status": "Executed successfully",
  "changes": [
    {
      "address": "0xb258b91eee04111039320a85b0c24a2dd433909e14a6b5c32ee722e0fdecfddc",
      "data": {
        "type": "0x1::coin::CoinStore<0x1337::my_coin::MyCoin>",
        "data": {
          "coin": {
            "value": "1000"
          },
          "deposit_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
                "creaton_num": "2",
              }
            }
          },
          ...
        }
      },
      "type": "write_resource"
    },
    ...
  ],
  "sender": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
  "sequence_number": "0",
  "max_gas_amount": "2000",
  "gas_unit_price": "110",
  "expiration_timestamp_secs": "1660616127",
  "payload": {
    "function": "0x1::coin::transfer",
    "type_arguments": [
      "0x1337::my_coin::MyCoin"
    ],
    "arguments": [
      "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
      "1000"
    ],
    "type": "entry_function_payload"
  },
  "events": [
    {
      "key": "0x0300000000000000810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
      "guid": {
        "id": {
          "addr": "0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b",
          "creation_num": "3"
          }
        }
      },
      "sequence_number": "0",
      "type": "0x1::coin::WithdrawEvent",
      "data": {
        "amount": "1000"
      }
    },
    {
      "key": "0x02000000000000005098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
      guid": {
        "id": {
          "addr": "0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e",
          "creation_num": "2"
          }
        }
      },
      "sequence_number": "0",
      "type": "0x1::coin::DepositEvent",
      "data": {
        "amount": "1000"
      }
    }
  ],
  "timestamp": "1660615531147935",
  "type": "user_transaction"
}
```

There are three balance changes in this transaction:
1. A withdrawal of `1000` of `0x1337::my_coin::MyCoin` from the transaction sending account `0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b`
2. A deposit of `1000` of `0x1337::my_coin::MyCoin` to receiving account `0x5098df8e7969b58ab3bd2d440c6203f64c60a1fd5c08b9d4abe6ae4216246c3e`
3. A gas fee `2200` of `0x1::aptos_coin::AptosCoin` from the sending account `0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b`

To retrieve the withdrawal information:
1. Scan the `changes` for `0x1::coin::CoinStore<CoinType>`.  Note the `CoinType` is a generic signifying which coin is stored in the store.  In this example, the `CoinType` is `0x1337::my_coin::MyCoin`.
2. Retrieve the `guid` for `withdraw_events`. In this example, the `guid` contains `addr` `0x810026ca8291dd88b5b30a1d3ca2edd683d33d06c4a7f7c451d96f6d47bc5e8b` and `creation_num` `3`.
3. Scan for events with this `guid` and extract the event associated with it.  In this example, it is the `0x1::coin::WithdrawEvent`.
4. Note the `amount` field will be the number of `CoinType` removed from the account in the `guid`. In this example, it is `1000`.

To retrieve the deposit information, it's the same as withdrawal except:
1. The `guid` used is under `deposit_events`
2. The `amount` will be a positive increase on the account's balance.
3. The event's name will be: `0x1::coin::DepositEvent`

To retrieve the gas fee:
1. The `gas_used` field must be multiplied times the `gas_unit_price`.  In this example, `gas_used=20` and `gas_unit_price=110` so the total gas coins withdrawn is `2200`.
2. Gas is always: `0x1::aptos_coin::AptosCoin`

To retrieve information about the number of decimals of the coin:
1. You can retrieve the number of decimals for a coin via its: `0x1::coin::CoinInfo<CoinType>`
2. This will be located at the address of the coin type.  In this example, you would need to look up `0x1::coin::CoinInfo<0x1337::my_coin::MyCoin>` at address `0x1337`.

:::tip
If you always use the events in this manner, you won't miss any balance changes for an account.
By monitoring the events, you will find all balance changes in the `0x1::coin::CoinStore`:
1. Coin mints
2. Coin burns
3. Coin transfers
4. Staking coins
5. Withdrawing staked coins
6. Transfers not derived from `coin::transfer`

:::

To create some sample data to explore, conduct ["Your first transaction"](../tutorials/your-first-transaction).

To learn more about coin creation, make ["Your First Coin"](../tutorials/your-first-coin).

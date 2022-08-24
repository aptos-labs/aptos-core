---
title: "Your First Move Module using the CLI"
slug: "first-move-module-cli"
sidebar_position: 2
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Move Module

This tutorial details how to compile, test, publish and interact with Move Modules on the Aptos Blockchain. The steps are:

1. [Install the CLI from Git][install_cli]
2. Compile and test a Move module
3. Publish a Move module to the Aptos Blockchain
4. Interact with a Move Module
5. Understand the code


## Prepare the CLI environment

After installing the CLI from Git, prepare your environment for interacting with Aptos by creating and funding an account. Begin by starting a new terminal.

1. Initialize a new local account: `aptos init`. This will output:
```
Enter your rest endpoint [Current: None | No input: https://fullnode.devnet.aptoslabs.com/v1]

No rest url given, using https://fullnode.devnet.aptoslabs.com/v1...
Enter your faucet endpoint [Current: None | No input: https://faucet.devnet.aptoslabs.com | 'skip' to not use a faucet]

No faucet url given, using https://faucet.devnet.aptoslabs.com...
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]

No key given, generating key...
Account a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a!  Run `aptos help` for more information about commands
{
  "Result": "Success"
}
```
2. Now fund this account: `aptos account fund-with-faucet --account a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a`
```
{
  "Result": "Added 10000 coins to account a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a"
}
```

## Compile and test the module

There are many example Move modules in the [aptos-core/aptos-move/move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples) directory.

Load a terminal and change directories into the `hello_blockchain` directory: `cd aptos-core/aptos-move/move-examples`.

To build the module run: `aptos move build --named-addresses hello_blockchain=0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a`.
To test the module run: `aptos move test --named-addresses hello_blockchain=0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a`.

The CLI entry must contain `--named-addresses` because the `Move.toml` file leaves this as undefined:

```toml
[addresses]
hello_blockchain = "_"
```

In order to prepare the module for the account created in the previous step, we specify that the named address `hello_blockchain` is set to our account address.

## Publish the Move module

Now that the code can be compiled and tests pass, let's publish the module to the account created for this tutorial:
`aptos move publish --named-addresses hello_blockchain=0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a`

```
package size 1631 bytes
{
  "Result": {
    "transaction_hash": "0x45d682997beab297a9a39237c588d31da1cd2c950c5ab498e37984e367b0fc25",
    "gas_used": 13,
    "gas_unit_price": 1,
    "pending": null,
    "sender": "a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a",
    "sequence_number": 8,
    "success": true,
    "timestamp_us": 1661320216343795,
    "version": 3977,
    "vm_status": "Executed successfully"
  }
}
```

At this point, the module is now stored on the account.

## Interact with the module

Move modules expose access points or `entry functions`. These can be called via transactions. The CLI allows for seamless access to these. `hello_blockchain` exposes a `set_message` entry function that takes in a `string`. This can be called via the CLI:

```
aptos move run \
--function-id '0x6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac::message::set_message' \
--args 'string:hello, blockchain'
```

Upon success, the CLI will print out the following:

```
{
  "Result": {
    "transaction_hash": "0x1fe06f61c49777086497b199f3d4acbee9ea58976d37fdc06d1ea48a511a9e82",
    "gas_used": 1,
    "gas_unit_price": 1,
    "pending": null,
    "sender": "6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac",
    "sequence_number": 1,
    "success": true,
    "timestamp_us": 1661320878825763,
    "version": 5936,
    "vm_status": "Executed successfully"
  }
}
```

The `set_message` modifies the `hello_blockchain` `MessageHolder` resource. A resource is a data structure that is stored in [global storage](https://move-language.github.io/move/structs-and-resources.html#storing-resources-in-global-storage). The resource can be read by querying the following REST API:

`https://fullnode.devnet.aptoslabs.com/v1/accounts/6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac/resource/0x6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac::message::MessageHolder`

Which after the first exectuion contains the following:

```
{
  "type":"0x6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac::message::MessageHolder",
  "data":{
    "message":"hello, blockchain",
    "message_change_events":{
      "counter":"0",
      "guid":{
        "id":{
          "addr":"0x6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac",
          "creation_num":"3"
        }
      }
    }
  }
}
```

Notice the `message` field contains `hello, blockchain`.

Each succesful call to `set_message` after the first call results in an update to `message_change_events`. The `message_change_events` for a given account can be accessed via the REST API: 

`http://127.0.0.1:8080/v1/accounts/6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac/events/0x6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac::message::MessageHolder/message_change_events`

Where after a call to set the message to `hello, blockchain, again`, the event stream would contain the following:
```
[
  {
    "version":"8556",
    "key":"0x03000000000000006dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac",
    "sequence_number":"0","type":"0x6dcdbfbbb2a1f5d2cd9b8f78b9ec32feaa0170db64ccfc02442af5384f0439ac::message::MessageChangeEvent",
    "data":{
      "from_message":"hello, blockchain",
      "to_message":"hello, blockchain, again"
    }
  }
]
```

:::tip

Other accounts can reuse the published module by calling the exact same function as in this example. It is left as an exerciser to validate this.

:::

[account_basics]: /concepts/basics-accounts
[alice_account_rest]: https://fullnode.devnet.aptoslabs.com/v1/accounts/a52671f10dc3479b09d0a11ce47694c0/
[bob_account_explorer]: https://explorer.devnet.aptos.dev/account/ec6ec14e4abe10aaa6ad53b0b63a1806
[install_cli]: /cli-tools/aptos-cli-tool/install-aptos-cli
[rest_spec]: https://fullnode.devnet.aptoslabs.com/v1/spec#/

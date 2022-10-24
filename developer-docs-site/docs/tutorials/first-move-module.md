---
title: "Your First Move Module"
slug: "first-move-module"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Move Module

This tutorial details how to compile, test, publish and interact with Move modules on the Aptos blockchain. The steps in summary are:

1. Install the precombiled binary for the Aptos CLI.
2. Create an account on the Aptos blockchain and fund it.
3. Compile and test a Move module.
4. Publish a Move module to the Aptos blockchain.
5. Interact with a Move module.

## Step 1: Install the CLI

[Install the precombiled binary for the Aptos CLI][install_cli].

---

## Step 2: Create an account and fund it 

After installing the CLI binary, next step is to create and fund an account on the Aptos blockchain. 

1. Begin by starting a new terminal and run the below command to initialize a new local account: 

```bash
aptos init
```

The output will be similar to below. 
```text
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

The account address in the above output:  `a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a` is your new account, and is aliased as the profile `default`. This account address will be different for you as it is generated randomly. From now on, either `default` or `0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a` are interchangeable.

2. Now fund this account by running this command: 

```bash
aptos account fund-with-faucet --account default
```
You will see an output similar to the below:
```
{
  "Result": "Added 10000 coins to account a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a"
}
```

---

## Step 3: Compile and test the module

Several example Move modules are available in the [aptos-core/aptos-move/move-examples](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples) directory. Open a terminal and change directories into the `hello_blockchain` directory: 

```bash
cd aptos-core/aptos-move/move-examples/hello_blockchain
```

Run the below command to compile the `hello_blockchain` module: 

```bash
aptos move compile --named-addresses hello_blockchain=default
```

To test the module run: 

```bash
aptos move test --named-addresses hello_blockchain=default
```

The CLI entry must contain `--named-addresses` because the [`Move.toml`](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/move-examples/hello_blockchain/Move.toml) file leaves this as undefined (see below). To prepare the module for the account created in the previous step, we specify that the named address `hello_blockchain` is set to our account address, using the `default` profile alias.

```toml
[addresses]
hello_blockchain = "_"
```

---

## Step 4: Publish the Move module

After the code was compiled and tested, we can publish the module to the account created for this tutorial. Run this below command:

```bash
aptos move publish --named-addresses hello_blockchain=default
```

You will see the output similar to the below:

```bash
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

At this point, the module is now stored on the account in the Aptos blockchain.

---

## Step 5: Interact with the Move module

Move modules expose access points, also referred as `entry functions`. These access points can be called via transactions. The CLI allows for seamless access to these access points. The example Move module `hello_blockchain` exposes a `set_message` entry function that takes in a `string`. This can be called via the CLI:

```bash
aptos move run \
  --function-id 'default::message::set_message' \
  --args 'string:hello, blockchain'
```

Upon success, the CLI will print out the following:

```json
{
  "Result": {
    "transaction_hash": "0x1fe06f61c49777086497b199f3d4acbee9ea58976d37fdc06d1ea48a511a9e82",
    "gas_used": 1,
    "gas_unit_price": 1,
    "pending": null,
    "sender": "a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a",
    "sequence_number": 1,
    "success": true,
    "timestamp_us": 1661320878825763,
    "version": 5936,
    "vm_status": "Executed successfully"
  }
}
```

The `set_message` function modifies the `hello_blockchain` `MessageHolder` resource. A resource is a data structure that is stored in [global storage](https://move-language.github.io/move/structs-and-resources.html#storing-resources-in-global-storage). The resource can be read by querying the following REST API:

```bash

https://fullnode.devnet.aptoslabs.com/v1/accounts/a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a/resource/0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a::message::MessageHolder
```

which, after the first execution contains the following:

```json
{
  "type":"0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a::message::MessageHolder",
  "data":{
    "message":"hello, blockchain",
    "message_change_events":{
      "counter":"0",
      "guid":{
        "id":{
          "addr":"0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a",
          "creation_num":"3"
        }
      }
    }
  }
}
```

Notice that the `message` field contains `hello, blockchain`.

Each successful call to `set_message` after the first call results in an update to `message_change_events`. The `message_change_events` for a given account can be accessed via the REST API: 

```bash
http://127.0.0.1:8080/v1/accounts/0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a/events/0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a::message::MessageHolder/message_change_events
```

where, after a call to set the message to `hello, blockchain, again`, the event stream would contain the following:
```json
[
  {
    "version":"8556",
    "key":"0x0300000000000000a345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a",
    "sequence_number":"0","type":"0xa345dbfb0c94416589721360f207dcc92ecfe4f06d8ddc1c286f569d59721e5a::message::MessageChangeEvent",
    "data":{
      "from_message":"hello, blockchain",
      "to_message":"hello, blockchain, again"
    }
  }
]
```

:::tip

Other accounts can reuse the published module by calling the exact same function as in this example. It is left as an exercise to the reader.

:::

[account_basics]: /concepts/basics-accounts
[alice_account_rest]: https://fullnode.devnet.aptoslabs.com/v1/accounts/a52671f10dc3479b09d0a11ce47694c0/
[bob_account_explorer]: https://explorer.aptoslabs.com/account/ec6ec14e4abe10aaa6ad53b0b63a1806
[install_cli]: /cli-tools/aptos-cli-tool/install-aptos-cli#download-precompiled-binary
[rest_spec]: https://fullnode.devnet.aptoslabs.com/v1/spec#/

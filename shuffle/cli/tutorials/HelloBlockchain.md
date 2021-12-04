# Hello Blockchain Tutorial

Welcome to Hello Blockchain tutorial! Our goal is to create a Message module
that you can use to set arbitrary message on your account message resource. We
will walk you through the steps of creating a new project to set a "hello
blockchain" message using Shuffle and Move.

## Create a New Project

To start a new Move project, run`shuffle new <path-to-new-project>`, which will
create a project directory with the given path name and a template to start Move
development.

Example: `shuffle new /tmp/helloblockchain`

Output:

```
Creating shuffle project in /tmp/helloblockchain
Copying Examples...
Generating Typescript Libraries...
Building helloblockchain/main...
BUILDING MoveStdlib
BUILDING Message
```

## Run a Node

Run `shuffle node` to deploy a local test node. Run this node on a separate
terminal. If you are testing against a remote node, you can skip this step.

Output:

```
Creating node config in /Users/sunmilee/.shuffle
Completed generating configuration:
	Log file: "/Users/sunmilee/.shuffle/nodeconfig/validator.log"
	Config path: "/Users/sunmilee/.shuffle/nodeconfig/0/node.yaml"
	Diem root key path: "/Users/sunmilee/.shuffle/nodeconfig/mint.key"
	Waypoint: 0:4adac71a66fe1725c53735156d7b4467bf52915c082f00d4db3cb980135335c7
	ChainId: TESTING
	JSON-RPC endpoint: 0.0.0.0:8080
	REST API endpoint: 0.0.0.0:8080
	Stream-RPC enabled!
	FullNode network: /ip4/0.0.0.0/tcp/7180

	Lazy mode is enabled

Diem is running, press ctrl-c to exit
```

## Create an Account

In order to deploy your modules, you must have an account on-chain. Run
`shuffle account` to create accounts on the default localhost network.

Note: `shuffle account` creates 2 accounts on-chain. The first account is the default account you will use in all your shuffle commands. All the modules you write will be published by this first account. The second account allows you to test transfer/p2p scenarios.

Output:

```
Connecting to http://0.0.0.0:8080...
Creating a new account onchain...
Successfully created account 24163AFCC6E33B0A9473852E18327FA9
Private key: 2f17c7a5f1175945aceb430f97e6413a89ce8f951e668283153c8f9f7e413bb6
Public key: a831c99b7a599965b716eb35b33d813342f028cdb5f0f0c068d7172055ecfff5
Creating a new account onchain...
Successfully created account D770294D6EEAB56836AB02735083E4EC
Private key: 6204190004b94c89dbfc993c18f91344fb7f5058bb9ce8e9a0ab79dac9d6592f
Public key: b1ca8b33235bf1f01b3e9d0278555858acb0a2d7cc1c2f428643fff18ff0e5ce
```

## Write a Module

Now you are ready to write your own modules. To start, create a move file in
`project_directory/main/sources`. There is an example module `Message.move` that
sets a message to `MessageHolder` move resource that you can play with.

## Deploy Your Module

Once you have your move module, you will need to deploy your module on-chain
using a transaction. `cd` into your project directory and run `shuffle deploy`.
This will compile all move modules inside the `helloblockchain/main` directory
and deploy them on-chain.

Output:

```
Using Public Key a831c99b7a599965b716eb35b33d813342f028cdb5f0f0c068d7172055ecfff5
Sending txn from address 0x24163afcc6e33b0a9473852e18327fa9
Building /Users/sunmilee/diem/helloblockchain/main...
CACHED MoveStdlib
CACHED Message
Skipping Module: 00000000000000000000000000000001::Hash
Skipping Module: 00000000000000000000000000000001::Signer
Skipping Module: 00000000000000000000000000000001::Vector
Skipping Module: 00000000000000000000000000000001::Errors
Skipping Module: 00000000000000000000000000000001::BitVector
Skipping Module: 00000000000000000000000000000001::Capability
Skipping Module: 00000000000000000000000000000001::FixedPoint32
Skipping Module: 00000000000000000000000000000001::Option
Skipping Module: 00000000000000000000000000000001::BCS
Skipping Module: 00000000000000000000000000000001::GUID
Skipping Module: 00000000000000000000000000000001::Event
Deploying Module: 24163AFCC6E33B0A9473852E18327FA9::Message
```

## Explore with Console

Once you have your module deployed, you can use `shuffle console` to enter a
typescript REPL. Enter the REPL inside your project folder and try running the
below commands:

Set the message field inside MessageHolder resource of your account to "hello
blockchain"

```
> await main.setMessageScriptFunction("hello blockchain");
{
  type: "pending_transaction",
  hash: "0x0d60430a2733e701fae009b88a61a4725e956c7a309d6344d05bb0a2ef46786e",
  sender: "0x825b47b8fd2b30cf37c0e58579a78bc8",
  sequence_number: "3",
  max_gas_amount: "1000000",
  gas_unit_price: "0",
  gas_currency_code: "XUS",
  expiration_timestamp_secs: "99999999999",
  payload: {
    type: "script_function_payload",
    function: "0x825b47b8fd2b30cf37c0e58579a78bc8::Message::set_message",
    type_arguments: [],
    arguments: [ "0x68656c6c6f20626c6f636b636861696e" ]
  },
  signature: {
    type: "ed25519_signature",
    public_key: "0x171b9cd908c329b2b3f091729799b74cba8a25d7075067022bbc15f1faa02303",
    signature: "0x2441dab7e423806041f55cc74fda3af79be019973a446e6f92cf15d281fc6c363fc53cb348c996db9155ec7a6984c93b46..."
  }
}
```

Get the latest account transactions to see if your transaction was executed
successfully

```
> await devapi.accountTransactions()
[
  ......
  {
    type: "user_transaction",
    version: "255",
    hash: "0x0d60430a2733e701fae009b88a61a4725e956c7a309d6344d05bb0a2ef46786e",
    state_root_hash: "0x58ba3634ed95a5232a575c1af7de043351ee225968b8b1efdc99bbd676b313ff",
    event_root_hash: "0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
    gas_used: "36",
    success: true,
    vm_status: "Executed successfully",
    sender: "0x825b47b8fd2b30cf37c0e58579a78bc8",
    sequence_number: "3",
    max_gas_amount: "1000000",
    gas_unit_price: "0",
    gas_currency_code: "XUS",
    expiration_timestamp_secs: "99999999999",
    payload: {
      type: "script_function_payload",
      function: "0x825b47b8fd2b30cf37c0e58579a78bc8::Message::set_message",
      type_arguments: [],
      arguments: [ "0x68656c6c6f20626c6f636b636861696e" ]
    },
    signature: {
      type: "ed25519_signature",
      public_key: "0x171b9cd908c329b2b3f091729799b74cba8a25d7075067022bbc15f1faa02303",
      signature: "0x2441dab7e423806041f55cc74fda3af79be019973a446e6f92cf15d281fc6c363fc53cb348c996db9155ec7a6984c93b46..."
    },
    events: []
  }
]
```
Print all the resources in an account

```
> await devapi.resources()
[
  { type: "0x1::GUID::Generator", data: { counter: "5" } },
  { type: "0x1::VASP::ParentVASP", data: { num_children: "0" } },
  { type: "0x1::Roles::RoleId", data: { role_id: "5" } },
  { type: "0x1::VASPDomain::VASPDomains", data: { domains: [] } },
  {
    type: "0x1::DiemAccount::Balance<0x1::XUS::XUS>",
    data: { coin: { value: "0" } }
  },
  {
    type: "0x1::DiemAccount::DiemAccount",
    data: {
      authentication_key: "0xd1a99d23710aaf0a05c70650623aa67b825b47b8fd2b30cf37c0e58579a78bc8",
      key_rotation_capability: { vec: [Array] },
      received_events: { counter: "0", guid: [Object] },
      sent_events: { counter: "0", guid: [Object] },
      sequence_number: "4",
      withdraw_capability: { vec: [Array] }
    }
  },
  { type: "0x1::AccountFreezing::FreezingBit", data: { is_frozen: false } },
  {
    type: "0x1::DualAttestation::Credential",
    data: {
      base_url: "0x",
      base_url_rotation_events: { counter: "0", guid: [Object] },
      compliance_key_rotation_events: { counter: "0", guid: [Object] },
      compliance_public_key: "0x",
      expiration_date: "18446744073709551615",
      human_name: "0x"
    }
  },
  {
    type: "0x825b47b8fd2b30cf37c0e58579a78bc8::Message::MessageHolder",
    data: {
      message: "hello blockchain",
      message_change_events: { counter: "0", guid: [Object] }
    }
  }
]
```

Use decodedMessages to check account message:

```
> await main.decodedMessages()
[ "hello blockchain" ]
```

Update message triggers an event, use `main.messageEvents` to find out all update events:

```
> await main.setMessageScriptFunction("hello again");
{
  type: "pending_transaction",
  hash: "0x262cb8ae79e26f616ba7faf43927b4011b85c366084d5ddbfa15123b5eb2be07",
  sender: "0x825b47b8fd2b30cf37c0e58579a78bc8",
  sequence_number: "3",
  max_gas_amount: "1000000",
  gas_unit_price: "0",
  gas_currency_code: "XUS",
  expiration_timestamp_secs: "99999999999",
  payload: {
    type: "script_function_payload",
    function: "0x825b47b8fd2b30cf37c0e58579a78bc8::Message::set_message",
    type_arguments: [],
    arguments: [ "0x68656c6c6f20616761696e" ]
  },
  signature: {
    type: "ed25519_signature",
    public_key: "0x171b9cd908c329b2b3f091729799b74cba8a25d7075067022bbc15f1faa02303",
    signature: "0xa10c5a63ff6a474b2bbfeaaca366860ffddc12e4b51d99a14327afa651ab79e9ab21e7f1634a12c9045f47bb060addd2ee..."
  }
}

> await main.messageEvents();
[
  {
    key: "0x0400000000000000825b47b8fd2b30cf37c0e58579a78bc8",
    sequence_number: "0",
    type: "0x825b47b8fd2b30cf37c0e58579a78bc8::Message::MessageChangeEvent",
    data: { from_message: "hello blockchain", to_message: "hello again" }
  }
]
```

## Write E2E Tests

You can also test your module by writing unit tests and integration tests. See
`helloblockchain/e2e/message.test.ts` for an example of an end to end test for
the Message module,

There are different subcommands available:

- `shuffle test all` runs both unit tests and end to end tests
- `shuffle test e2e` runs all end to end tests

Output:

```
Connecting to http://127.0.0.1:8080/...
Creating a new account onchain...
Account already exists: 34A0BBEA1989B7D5AB57902B2BE949DE
Public key: 7bc846dd119be8af757bc4ae9f7b7854876d04d6c83635df90771ed92a4b278e
Creating a new account onchain...
Account already exists: 34A0BBEA1989B7D5AB57902B2BE949DE
Public key: 7bc846dd119be8af757bc4ae9f7b7854876d04d6c83635df90771ed92a4b278e
Using Public Key a831c99b7a599965b716eb35b33d813342f028cdb5f0f0c068d7172055ecfff5
Sending txn from address 0x24163afcc6e33b0a9473852e18327fa9
Building /Users/sunmilee/diem/helloblockchain/main...
CACHED MoveStdlib
CACHED Message
Skipping Module: 00000000000000000000000000000001::Hash
Skipping Module: 00000000000000000000000000000001::Signer
Skipping Module: 00000000000000000000000000000001::Vector
Skipping Module: 00000000000000000000000000000001::Errors
Skipping Module: 00000000000000000000000000000001::BitVector
Skipping Module: 00000000000000000000000000000001::Capability
Skipping Module: 00000000000000000000000000000001::FixedPoint32
Skipping Module: 00000000000000000000000000000001::Option
Skipping Module: 00000000000000000000000000000001::BCS
Skipping Module: 00000000000000000000000000000001::GUID
Skipping Module: 00000000000000000000000000000001::Event
Deploying Module: 24163AFCC6E33B0A9473852E18327FA9::Message

Check file:///Users/sunmilee/diem/helloblockchain/e2e/message.test.ts
Loading Project /Users/sunmilee/diem/helloblockchain
Sender Account Address 0x34a0bbea1989b7d5ab57902b2be949de
"helpers", "devapi", "context", "main", "codegen", "help" top level objects available
Run "help" for more information on top level objects
Connecting to Node http://127.0.0.1:8080/
{ chain_id: 4, ledger_version: "99", ledger_timestamp: "1637302745588328" }

running 2 tests from file:///Users/sunmilee/diem/helloblockchain/e2e/message.test.ts
test Test Assert ... ok (12ms)
test Ability to set message ... ok (2s)

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out (5s)
```

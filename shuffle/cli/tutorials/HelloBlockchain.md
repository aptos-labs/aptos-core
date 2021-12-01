#Hello Blockchain Tutorial

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

Run `shuffle node` to deploy a local test node. If you are testing against a
remote node, you can skip this step.

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

```
// Set the message field inside MessageHolder resource of your account to "hello blockchain"
await main.setMessageScriptFunction("hello blockchain");

// Get the latest account transactions to see if your transaction was executed successfully
await devapi.accountTransactions()

// Get account resources
await devapi.resources()

// Use decodedMessages to check that message was set to "hello blockchain"
main.decodedMessages()
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

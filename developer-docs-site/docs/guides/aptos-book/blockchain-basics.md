---
title: "Blockchain Basics"
---

# Blockchain Basics

This section introduces a few key concepts around Aptos to help developers understand their development process. This is an abbreviated version of the [Aptos Blockchain Deep Dive](../../../concepts/blockchain).

## Accounts and Wallets

Consider a scenario where there are two blockchain users, Alice and Bob. Alice wants to transfer Bob some of the Aptos utility token, `AptosCoin`. A utility token is the basic resource of utility in a blockchain, in the case of Aptos, it is used to cover execution and storage fees as a base deterrent against denial of service attacks.

Much like other accounts, in Aptos, each account is represented by an account address or a 32-byte number derived from the public key in a private, public key pair. Thus any key pair generated is a potential account on-chain. An account on-chain is created either explicitly via a create function or implicitly by transferring `AptosCoin` to that account.

For example, at some time, long ago, Alice went to a centralized exchange, acquired some `AptosCoin` and transferred the `AptosCoin` to her account address specified managed her self-custodial wallet, creating an on-chain account. Bob, like Alice, has a self-custodial wallet, though unlike Alice his account has never interacted with the blockchain and will be created after Alice transfers him coin.

In general, wallets typically contain multiple accounts. In fact, the accounts represented by Alice and Bob may be from the same wallet and the same person.

## Transactions and Interfaces

In order to send Bob some funds, Bob would need to first give Alice his account address or alternatively a human-readable name from [Aptos Names](https://www.aptosnames.com/). As most wallets support both, Alice will find it more convenient to transfer funds to `bob.apt` in contrast to something like `0x4c54e23a8064f9636c83bd1490a782f5b421ac7ea48dd26ebef4f030b77dd252`.

Alice then opens her wallet, enters the identifier for Bob’s account and specifies the amount to transfer. This in turns constructs a transaction for the blockchain that an account owned by Alice has authenticated in order to call a function called `0x1::aptos_account::transfer` with two parameters, the address and amount of Aptos coin to transfer. Alice’s wallet then calls into the Aptos REST API service and submits the transaction to the blockchain. Note, all applications speak to the Aptos blockchain via the REST API for both reading blockchain state and submitting transactions, read and write services, respectively.

Eventually the transaction makes it way toward the core of the blockchain that executes and certifies transactions; this core is called the validator network. In order to get to the validator network, the REST API, which typically runs on a public fullnode, transmits the transaction to a validator fullnode, which acts as a gateway to the validator network. The validators execute each transaction within the Move VM taking the transaction as well as the previous blockchain state as input and commit the output as the new blockchain state. You can learn more in the [Aptos Blockchain Deep Dive](../../../concepts/blockchain).

## Preparing to Build

Before we move forward, [install the Aptos CLI](../../../tools/install-cli/) as it will be used for running the test code.

The rest of this guide will assume that the Aptos CLI (`aptos`) is available globally, that is calling `aptos` at a terminal executes the Aptos CLI.

Before starting into Move, you need to setup your developer environment. With the Aptos CLI, you can run a fully functioning Aptos local testnet making application development seamless!

### Starting the Testnet

To start a local testnet, execute the following command:

```bash
aptos node run-local-testnet --with-faucet --force-restart --assume-yes
```

This guide always assumes a new empty state, hence the inclusion of `--force-restart`. You can omit this flag if you want to use existing state from a previous local testnet run.

The output from the command should be similar to the following:

```bash
Completed generating configuration:
        Log file: "/home/davidiw/aptos/aptos-core/.aptos/testnet/validator.log"
        Test dir: "/home/davidiw/aptos/aptos-core/.aptos/testnet"
        Aptos root key path: "/home/davidiw/aptos/aptos-core/.aptos/testnet/mint.key"
        Waypoint: 0:e881e7134588985689c47a8c5c6a15dd4d95f72e5e68ec6246a2f3a6d65ddc45
        ChainId: testing
        REST API endpoint: http://0.0.0.0:8080
        Metrics endpoint: http://0.0.0.0:9101/metrics
        Aptosnet fullnode network endpoint: /ip4/0.0.0.0/tcp/6181

Aptos is running, press ctrl-c to exit

Faucet is running. Faucet endpoint: http://0.0.0.0:8081
```

### Prepare a User

As most of our interaction will be with the local testnet, we’ll create two identities:

```bash
# Create alice's profile
aptos init \
  --profile alice \
  --rest-url http://localhost:8080 \
  --faucet-url http://localhost:8081 \
  --network custom

# Create bob's profile
aptos init \
  --profile alice \
  --rest-url http://localhost:8080 \
  --faucet-url http://localhost:8081 \
  --network custom \
  --skip-faucet
```

The addresses and private keys will persist for all future testnets, so we will not need to call init again. However, to ensure freshness, we will typically start with a fresh testnet.

### Acquiring More Funds

While the CLI tool by default initializes accounts on-chain by calling an available faucet, this may be insufficient and the account may need more coins. To acquire more coins on a network with a faucet perform the following operation:

```bash
$ aptos account fund-with-faucet --profile alice --account alice

{
  "Result": "Added 100000000 Octas to account d20f305e3090a24c00524604dc2a42925a75c67aa6020d33033d516cf0878c4a"
}
```

### Checking Balances

The balances can be checked by querying the following endpoint:

```bash
$ aptos move view --profile alice \
    --function-id 0x1::coin::balance \
    --type-args 0x1::aptos_coin::AptosCoin \
    --args address:alice

{
  "Result": [
    "200000000"
  ]
}
```

We just queried called a `view` function within Move called `balance` in the module `coin` at the address `0x1`. The nuances of this call will become clear as we progress through our journey through Aptos and Move.

Because Bob's account has yet to be created, querying it results in the following message

```bash
$ aptos move view --profile alice \
    --function-id 0x1::coin::balance \
    --type-args 0x1::aptos_coin::AptosCoin \
    --args address:alice

{
  "Error": "API error: API error Error(InvalidInput): Failed to execute function: VMError { major_status: ABORTED, sub_status: Some(393221), message: Some(\"0x0000000000000000000000000000000000000000000000000000000000000001::coin::balance at offset 6\"), exec_state: Some(ExecutionState { stack_trace: [] }), location: Module(ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier(\"coin\") }), indices: [], offsets: [(FunctionDefinitionIndex(1), 6)] }"
}
```

This roughly equates to a does not exist error.

### A Simple Transaction

To create Bob's account. Alice can trigger a transfer from her account to Bob's by calling a move function directly:

```bash
$ aptos move run --profile alice \
    --function-id 0x1::aptos_account::transfer \
    --args address:bob u64:1000
```

It will prompt you to confirm the following:
```bash
Do you want to submit a transaction for a range of [100900 - 151300] Octas at a gas unit price of 100 Octas? [yes/no] >
```

Where we respond with `yes`.

At which point, it outputs the details of the transaction:

```bash
{
  "Result": {
    "transaction_hash": "0x771bac014a8e6fe428f01d96250ff9c71ce4dee38356f29be97d0595c9bd4123",
    "gas_used": 1009,
    "gas_unit_price": 100,
    "sender": "d20f305e3090a24c00524604dc2a42925a75c67aa6020d33033d516cf0878c4a",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1686641065894063,
    "version": 2762,
    "vm_status": "Executed successfully"
  }
}
```

And then verify that the account balances are appropriately reflected by calling:
```bash
$ aptos move view --profile bob \
    --function-id 0x1::coin::balance \
    --type-args 0x1::aptos_coin::AptosCoin \
    --args address:bob

{
  "Result": [
    "1000"
  ]
}
```

Thus confirming that Bob's account was created and it now contains a balance of 1000.

Similarly, we can see Alice's account was debited the 1000 unit transfer along with the gas fee within the range that the API suggested:
```bash
$ aptos move view --profile alice \
    --function-id 0x1::coin::balance \
    --type-args 0x1::aptos_coin::AptosCoin \
    --args address:alice

{
  "Result": [
    "199898100"
  ]
}
```

Note, the details on gas amounts and various fields may differ across executions as many of these fields relate to the random configuration created by the blockchain state and account creation.

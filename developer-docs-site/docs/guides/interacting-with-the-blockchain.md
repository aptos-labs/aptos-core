---
title: "Interacting with the Aptos Blockchain"
slug: "interacting-with-the-aptos-blockchain"
---

# Interacting with the Aptos Blockchain

The Aptos blockchain uses the [Move][move_url] virtual machine (VM) for executing operations. While many blockchains implement a set of
native operations, Aptos delegates all operations to Move, including: account creation, fund transfer and publishing Move modules.
To support these operations, blockchains built on top of Move must provide a framework (akin to
an operating system for a computer or a minimal viable set of functions) for interacting with the blockchain. In this section, we discuss
these functions, exposed via the Aptos Framework's `script` functions.

This guide (in concert with the [Move module tutorial][your-first-move-module]) will unlock the minimal amount of information required to start building rich applications on top of the Aptos blockchain. Note: the Aptos Framework is under heavy development and this document may not
be up to date. The most recent framework can be found in the source code, [here][aptos_framework].

The core functions provided to users within the Aptos Framework include:
* Sending and receiving the network coin `Coin<AptosCoin>`
* Creating a new account
* Publishing a new Move module

Note: this document assumes readers are already familiar with submitting transactions, as described in the [Your first transaction tutorial][your-first-transaction].

## Sending and Receiving the network coin `Coin<AptosCoin>`

`Coin<AptosCoin>` is required for paying gas fees when submitting and executing transactions. `Coin<AptosCoin>` can be obtained by calling the Devnet Faucet. See the [Your first transaction tutorial][your-first-transaction] for an example.

The payload for instructing the blockchain to perform a transfer is:

```
{
  "type": "entry_function_payload",
  "function": "0x1::Coin::transfer",
  "type_arguments": ["0x1::aptos_coin::AptosCoin"],
  "arguments": [
    "0x737b36c96926043794ed3a0b3eaaceaf",
    "1000",
  ]
}
```

This instructs the VM to execute the `script` `0x1::Coin::transfer` with a type argument of 0x1::aptos_coin::AptosCoin. Type is required here as Coin is our standard module that can be used to create many types of Coins. See the [Your first coin tutorial][your-first-coin] for an example of creating a custom Coin. The first argument is the recipient address, `0x737b36c96926043794ed3a0b3eaaceaf`, and the second is the amount to transfer, `1000`. The sender address is the account
address that sent the transaction querying this `script`.

## Creating a new account

The payload for instructing the blockchain to create a new account is:

```
{
  "type": "entry_function_payload",
  "function": "0x1::AptosAccount::create_account",
  "type_arguments": [],
  "arguments": [
    "0x0c7e09cd9185a27104fa218a0b26ea88",
    "0xaacf87ae9d8a5e523c7f1107c668cb28dec005933c4a3bf0465ffd8a9800a2d900",
  ]
}
```

This instructs the Move virtual machine to execute the `script` `0x1::AptosAccount::create_account`. The first argument is the address of the account to create and the second is the authentication key pre-image (which is mentioned in [Accounts][accounts]). For single signer authentication, this is the public key concatenated with the `0` byte (or `pubkey_A | 0x00`). This is required to prevent account address land grabbing. The execution of this instruction verifies that the last 16-bytes of the authentication key are the same as the 16-byte account address. We are actively working on improving this API to support taking in a 32-byte account address that would eliminate concerns around land grabbing or account manipulation.

## Publishing a new Move module

The payload for publishing a new module is:

```
"type": "module_bundle_payload",
"modules": [
    {"bytecode": "0x..."},
],
```

This instructs the VM to publish the module bytecode under the sender's account. For a full length tutorial see [Your first move module][your-first-move-module].

It is important to note that the Move bytecode must specify the same address as the sender's account, otherwise the transaction will be rejected. For example, assuming account address `0xe110`, the Move module would need to be updated as such `module 0xe110::Message`, `module 0xbar::Message` would be rejected. Alternatively an aliased address could be used, such as `module HelloBlockchain::Message` but the `HelloBlockchain` alias would need to updated to `0xe110` in the `Move.toml` file. We are working with the Move team and planning on incorporating a compiler into our REST interface to mitigate this issue.

[accounts]: /concepts/basics-accounts
[your-first-coin]: /tutorials/your-first-coin
[your-first-move-module]: /tutorials/first-move-module
[your-first-transaction]: /tutorials/your-first-transaction
[move_url]: https://diem.github.io/move/
[aptos_framework]: https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/framework/aptos-framework/sources

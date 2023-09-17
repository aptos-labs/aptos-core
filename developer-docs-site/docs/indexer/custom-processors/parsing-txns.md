---
title: "Parsing Transactions"
---

# Parsing Transactions

<!--
Things to add:
- We should have tabs for each language that mentions helper functions for extracting the thing you want. For example, if the user is trying to extract the entry function arguments, there should be a function like `get_entry_function_arguments` and we show how to use it in each language and where it comes from in the SDK.
-->

Fundamentally an indexer processor is just something that consumes a stream of a transactions and writes processed data to storage. Let's dive into what a transaction is and what kind of information you can extract from one.

## What is a transaction?

A transaction is a unit of execution on the Aptos blockchain. If the execution of the program in a transaction (e.g. starting with an entry function in a Move module) is successful, the resulting change in state will be applied to the ledger. Learn more about the transaction lifecycle at [this page](/concepts/blockchain/#life-of-a-transaction).

There are four types of transactions on Aptos:
- Genesis
- Block metadata transactions
- State checkpoint transactions
- User transactions

The first 3 of these are internal to the system and are not relevant to most processors; we do not cover them in this guide.

Generally speaking, most user transactions originate from a user calling an entry function in a Move module deployed on chain, for example `0x1::coin::transfer`. In all other cases they originate from [Move scripts](/move/move-on-aptos/move-scripts). You can learn more about the different types of transactions [here](../../concepts/txns-states##types-of-transactions).

A user transaction that a processor handles contains a variety of information. At a high level it contains:
- The payload that was submitted.
- The changes to the ledger resulting from the execution of the function / script.

We'll dive into this in the following sections.

## What is important in a transaction?

### Payload

The payload is what the user submits to the blockchain when they wish to execute a Move function. Some of the key information in the payload is:
- The sender address
- The address + module name + function name of the function being executed.
- The arguments to the function.

There is other potentially interesting information in the payload that you can learn about at [this page](/concepts/txns-states#contents-of-a-transaction).

### Events

Events are emitted during the execution of a transaction. Each Move module can define its own events and choose when to emit the events during execution of a function.

For example, in Move you might have the following:
```rust
struct MemberInvitedEvent has store, drop {
    member: address,
}

public entry fun invite_member(member: address) {
    event::emit_event(
        &mut member_invited_events,
        MemberInvitedEvent { member },
    );
}
```

If `invite_member` is called, you will find the `MemberInvitedEvent` in the transaction.

:::tip Why emit events?
This is a good question! In some cases, you might find it unnecessary to emit events since you can just parse the writesets. However, sometimes it is quite difficult to get all the data you need from the different "locations" in the transaction, or in some cases it might not even be possible, e.g. if you want to index data that isn't included in the writeset. In these cases, events are a convenient way to bundle together everything you want to index.
:::

### Writesets

When a transaction executes, it doesn't directly affect on-chain state right then. Instead, it outputs a set of changes to be made to the ledger, called a writeset. The writeset is applied to the ledger later on after all validators have agreed on the result of the execution.

Writesets show the end state of the on-chain data after the transaction has occurred. They are the source of truth of what data is stored on-chain. There are several types of write set changes:

- Write module / delete module
- Write resource / delete resource
- Write table item / delete table item

<!-- Add more information about writesets, ideally once have the helper functions. -->

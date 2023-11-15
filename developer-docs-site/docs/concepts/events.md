---
title: "Events"
slug: "events"
---

Events are emitted during the execution of a transaction. Each Move module can define its own events and choose when to
emit the events upon execution of the module. Aptos Move starts with events with handle on mainnet launch, then switch
to module events since v1.7.

# Module Events

Since v1.7, Aptos Move released a new event framework, module events. Every event stream is identified by an event
struct type instead of a handle.
To define an event struct, just add attribute `#[event]` to a normal Move struct that has `drop + store` abilities. For
example,

```
/// 0xcafe::my_module_name
/// An example module event struct denotes a coin transfer.
#[event]
struct TransferEvent has drop, store {
    sender: address,
    receiver: address,
    amount: u64
}
```

for example,

```
// Define an event.
let event = TransferEvent {
    sender: 0xcafe,
    receiver: 0xface,
    amount: 100
};
// Emit the event just defined.
0x1::event::emit(event);
```

The module event has been emitted.

Example module events are available [here](https://explorer.aptoslabs.com/txn/682252266/events?network=testnet). Indices
0, 1, 2 are three module events of
type `0x66c34778730acbb120cefa57a3d98fd21e0c8b3a51e9baee530088b2e444e94c::event::MyEvent`. For module events compatible
with the old events, `Account Address`, `Creation Number` and `Sequence Number` are all set to dummy value 0.

## Access in Test

Module events are stored in a separate merkle tree called event accumulator for each transaction. Since it is apart from
state tree, MoveVM does not have read access to them when executing transaction in production. But in test, Aptos Move
adds two native functions to read emitted module events for better debugging experience.

```rust
/// Return all emitted module events with type T as a vector.
# [test_only]
public native fun emitted_events<T: drop + store>(): vector<T>;

/// Return true iff `msg` was emitted.
# [test_only]
public fun was_event_emitted<T: drop + store>(msg: & T): bool
```

Module event API is under construction to fetch events. The raw events api including the old event framework
in [graphql](https://aptos.dev/guides/system-integrators-guide/#production-network-access) is still available.

## Migration to Module Events

Since the release of module events, the old event framework involving `EventHandle` is deprecated due to the superiority
of module events in all aspects. The migration method is simple:

Emit a module event wherever an old event is emitted.

In this way, current projects can still use the ecosystem support for old event framework. When module events are fully
supported in the toolchain, those projects could seamlessly switch to module event stream and delete the code emitting
old events.

# Events with Handle (Deprecated)

This is the default event framework inherited from Libra/Diem era, where each event stream is identified by an event
handle, which is a Move struct that must be initialized before usage. Each handle consists of a GUID and a counter
assign sequence number to each event in this event stream.

For example, during a [coin transfer](../tutorials/first-transaction.md), both the sender and receiver's accounts will
emit `SentEvent` and `ReceivedEvent`, respectively. This data is stored within the ledger and can be queried via the
REST
interface's [Get events by event handle](https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_events_by_event_handle).

Assuming that an account `0xc40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd` had sent coins to another
account, the following query could be made to the REST
interface: `https://fullnode.devnet.aptoslabs.com/v1/accounts/c40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd/events/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>/withdraw_events`.
The output would be all `WithdrawEvent`s stored on that account, it would look like

```json
[
  {
    "key": "0x0000000000000000caa60eb4a01756955ab9b2d1caca52ed",
    "sequence_number": "0",
    "type": "0x1::coin::WithdrawEvent",
    "data": {
      "amount": "1000"
    }
  }
]
```

Each registered event has a unique `key`. The key `0x0000000000000000caa60eb4a01756955ab9b2d1caca52ed` maps to the
event `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>/sent_events` registered on
account `0xc40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd`. This key can then be used to directly make
event queries,
e.g., `https://fullnode.devnet.aptoslabs.com/v1/events/0x0000000000000000caa60eb4a01756955ab9b2d1caca52ed`.

These represent event streams, or a list of events with each entry containing a sequentially
increasing `sequence_number` beginning at `0`, a `type`, and `data`. Each event must be defined by some `type`. There
may be multiple events defined by the same or similar `type`s especially when using generics. Events have
associated `data`. The general principle is to include all data necessary to understand the changes to the underlying
resources before and after the execution of the transaction that changed the data and emitted the event.

[coin_transfer]: https://github.com/aptos-labs/aptos-core/blob/bdd0a7fe82cd6aab4b47250e5eb6298986777cf7/aptos-move/framework/aptos-framework/sources/coin.move#L412

[get_events]: https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_events_by_event_handle

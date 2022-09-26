---
title: "Events"
slug: "basics-events"
---

Events are emitted during the execution of a transaction. Each Move module can define its own events and choose when to emit the events upon execution of the module. For example, during a [coin transfer][coin_transfer], both the sender and receiver's accounts will emit `SentEvent` and `ReceivedEvent`, respectively. This data is stored within the ledger and can be queried via the REST interface's [Get events by event handle][get_events].

Assuming that an account `0xc40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd` had sent coins to another account, the following query could be made to the REST interface: `https://fullnode.devnet.aptoslabs.com/v1/accounts/c40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd/events/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>/withdraw_events`. The output would be all `WithdrawEvent`s stored on that account, it would look like 

```json
[{
  "key":"0x0000000000000000caa60eb4a01756955ab9b2d1caca52ed",
  "sequence_number":"0",
  "type":"0x1::coin::WithdrawEvent",
  "data":{
    "amount":"1000"
  }
}]
```

Each registered event has a unique `key`. The key `0x0000000000000000c40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd` maps to the event `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>/sent_events` registered on account `0xc40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd`. This key can then be used to directly make event queries, e.g., `https://fullnode.devnet.aptoslabs.com/v1/events/0000000000000000c40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd`.

These represent event streams, or a list of events with each entry containing a sequentially increasing `sequence_number` beginning at `0`, a `type`, and `data`. Each event must be defined by some `type`. There may be multiple events defined by the same or similar `type`s especially when using generics. Events have associated `data`. The general principle is to include all data necessary to understand the changes to the underlying resources before and after the execution of the transaction that changed the data and emitted the event.

[coin_transfer]: https://github.com/aptos-labs/aptos-core/blob/bdd0a7fe82cd6aab4b47250e5eb6298986777cf7/aptos-move/framework/aptos-framework/sources/coin.move#L412
[get_events]: https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_events_by_event_handle

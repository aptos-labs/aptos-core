---
title: "Events"
slug: "basics-events"
---

# Events

Events are emitted during the execution of a transaction. Each Move module can define its own events and choose when to emit the events upon execution of the module. 

## Example
For example, during a [coin transfer][coin_transfer], the sender's account will emit  `SentEvent` and the receiver's account will emit `ReceivedEvent`. This event data is stored within the ledger and can be queried via the REST API's [Get events by event handle][get_events] endpoint.

Assuming that an account `0xc40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd` had sent coins to another account, the following query could be made to the REST interface:

```bash
https://fullnode.devnet.aptoslabs.com/v1/accounts/c40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd/events/0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>/withdraw_events
```
The output would contain all `WithdrawEvent`s stored on that account, and looks like thhis: 

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

## Event key

Each registered event has a unique `key`. The key:
```text
0x0000000000000000c40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd
```
maps to the event:
```text
0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>/sent_events
```
registered on the account:
```text
0xc40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd
```
This key can then be used to directly make event queries. For example:
```bash
https://fullnode.devnet.aptoslabs.com/v1/events/0000000000000000c40f1c9b9fdc204cf77f68c9bb7029b0abbe8ad9e5561f7794964076a4fbdcfd
```

In response to the above query, the REST API endpoint will return event streams, or a list of events, with each entry containing:

- A sequentially increasing `sequence_number` beginning at `0`
- The `type` and `data`. 

Each event must be defined by some `type`. There may be multiple events defined by the same or similar `type`s especially when using generics. Events have associated `data`. 

:::tip Best practice
You should include all the data necessary to understand the changes to the underlying resources before and after the execution of the transaction that changed the data and emitted the event.
:::

[coin_transfer]: https://github.com/aptos-labs/aptos-core/blob/bdd0a7fe82cd6aab4b47250e5eb6298986777cf7/aptos-move/framework/aptos-framework/sources/coin.move#L412
[get_events]: https://fullnode.devnet.aptoslabs.com/v1/spec#/operations/get_events_by_event_handle

# Transaction Filter

## Overview

The goal of **transaction filtering** is to be able to save resources downstream of wherever filtering is used.
For this to be true, the filtering itself must be **fast and use minimal resources**, and so we do a few things:

1. We avoid clones, copies, etc as much as possible
2. We do a single pass over the transaction data

## Transaction Filtering

There are a few different parts of a transaction that are queryable:

1. The "root" level. This includes:
    - Transaction type
    - Success
2. User Transactions. Each user transaction has:
    - Sender
    - Payload: we only support the entry function payload
    - Entry function (address, module, name)
    - Entry function ID string
3. Events. Each event has:
    - Key
    - Type

### Usage & Examples

There are two different patterns for building a filter- you can either use the `TransactionFilterBuilder` or
the `TransactionFilter` struct directly.

The `TransactionFilterBuilder` is a more ergonomic way to build a filter, and is not significantly worse construction
performance, assuming this is being done infrequently.

```
use transaction_filter::filters::EventFilterBuilder;

let ef = EventFilterBuilder::default()
  .data("spins")
  .struct_type(
    MoveStructTagFilterBuilder::default()
      .address("0x0077")
      .module("roulette")
      .name("spin")
      .build()?,
  )
  .build()?;
```

The `TransactionFilter` struct is also available, but requires direct construction of the structs.

```
use transaction_filter::filters::EventFilter;

let ef = EventFilter {
  data: Some("spins".into()),
  struct_type: Some(MoveStructTagFilter {
    address: Some("0x0077".into()),
    module: Some("roulette".into()),
    name: Some("spin".into()),
  }),
};
```

Once you have some filters built, you can combine them with the boolean operators `and`, `or`, and `not`.

```
let trf = TransactionRootFilterBuilder::default()
  .success(true).build()?;

let utf = UserTransactionFilterBuilder::default()
  .sender("0x0011".into()).build()?;

let ef = EventFilterBuilder::default()
  .struct_type(
    MoveStructTagFilterBuilder::default()
      .address("0x0077")
      .module("roulette")
      .name("spin")
      .build()?,
  )
  .build()?;

// Combine filters using logical operators!
// (trf OR utf)
let trf_or_utf = BooleanTransactionFilter::from(trf).or(utf);
// ((trf OR utf) AND ef)
let query = trf_or_utf.and(ef);

let transactions: Vec<Transaction> = transaction_stream.next().await;
let filtered_transactions = query.filter_vec(transactions);
```

## API & Serialization

`BooleanTransactionFilter` is the top level filter struct, and it uses `serde` for serialization and deserialization.

This means we can use it across all of our projects, whether they be GRPC services, REST services, or CLI tools.

The above example can be serialized to JSON like so:

```json
{
  "and": [
    {
      "or": [
        {
          "type": "TransactionRootFilter",
          "success": true
        },
        {
          "type": "UserTransactionFilter",
          "sender": "0x0011"
        }
      ]
    },
    {
      "type": "EventFilter",
      "struct_type": {
        "address": "0x0077",
        "module": "roulette",
        "name": "spin"
      }
    }
  ]
}
```

Or, if you prefer, as yaml:

```yaml
---
and:
  - or:
      - type: TransactionRootFilter
        success: true
      - type: UserTransactionFilter
        sender: '0x0011'
  - type: EventFilter
    struct_type:
      address: '0x0077'
      module: roulette
      name: spin
```


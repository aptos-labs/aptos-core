---
title: "ANS FAQ"
id: "aptos-names-service-faq"
---

todo, this is a bit of a mess bc currently we recommend using aptosnames.com/api, but really we should use a fullnode

## Which query method should I use?
For now, query via aptosnames.com. SDK support will land soon for querying directly via the fullnode, which is a more reliable approach.

## How do I query name -> address via aptosnames.com/api?
See https://www.aptosnames.com/about/doc.

## How do I query address -> name via an aptosnames.com/api?
See https://www.aptosnames.com/about/doc.

## What name is returned if multiple names point to the same address?
TODO: Answer what the address -> name lookup returns.

## How do I query name -> address via a fullnode?
Coming soon!

TODO: Provide answers for curl + each of the SDKs (which should use the fullnode).

## How do I query address -> name via a fullnode?
Coming soon!

TODO: Provide answers for curl + each of the SDKs (which should use the fullnode) (once the contract change lands).

## How do I query name -> address via an indexer?
If you prefer to do it this way, you can do so with a query like this:
```
todo
```

Note that the fullnode approach is lower latency and has fewer dependencies though, so using a fullnode (which is what the SDKs use) is recommended.

## How do I query address -> name via an indexer?
If you prefer to do it this way, you can do so with a query like this:
```
todo
```

## Are the endpoints at aptosnames.com/api reliable?
Consider a lookup via aptosnames.com/api, for example this name to address lookup:
```ts
const name = "test";
const response = await fetch(`https://www.aptosnames.com/api/testnet/v1/address/${name}`);
const { address } = await response.json();
```

This takes the following path: Client -> aptosnames.com/api -> Google Cloud Functions -> Public Aptos Indexer. The indexer in turn relies on its own DB and fullnode.

So far this service has never gone down, but if you consider this too many potential points of failure, you may do a lookup via the fullnode directly (coming soon).

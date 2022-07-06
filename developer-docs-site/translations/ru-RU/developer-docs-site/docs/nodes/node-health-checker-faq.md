---
title: "Node Health Checker FAQ"
slug: "node-health-checker-faq"
sidebar_position: 10
---

import BlockQuote from "@site/src/components/BlockQuote";

# Node Health Checker FAQ
The Aptos Node Health Checker (NHC) is a tool Aptos offers to the community for a few different key use cases. For now you can see more about NHC, why we have it, how to run it, and more at the [Node Health Checker README](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/node-checker) in our repo.

The purpose of this FAQ is to help users understand why their node did not pass a particular evaluation from NHC. If you couldn't find the information you wanted in this FAQ, please [open an issue](https://github.com/aptos-labs/aptos-core/issues/new/choose) and we can add it. Even better, feel free to [open a PR](https://github.com/aptos-labs/aptos-core/pulls) and add the information yourself!

## How does the latency evaluator work?
You are likely here because you were given an evaluation result like this:
```
Average latency too high: The average latency was 1216ms, which is higher than the maximum allowed latency of 1000ms.
```

When faced with this error, you might see that the validation reports something like 1200ms above, but then when you `ping`, you see something more like 600ms. This difference comes from a misunderstanding in how our latency test works. When you `ping` an IP, the result you see is a single round trip (where the latency is RTT, round trip time). Our latency test is not doing an ICMP ping though, but timing a request to the API running on your node. In effect, this means we're timing 2 round trips, because it does the following:

1. SYN
2. SYNACK
3. ACK + Send HTTP request
4. Receive HTTP response

Because we must do a TCP handshake (one round trip) and then make an HTTP request (another round trip).

The reason we have the latency evaluator is to ensure we can maintain good network performance. In particular, if the latency too your node is too high, it will result in low TPS and high time to finality, both of which are very important to running a highly performant L1 blockchain. If you receive this error, you must work to try and improve the latency to your node, we already set high thresholds on this value with the understanding that nodes will be running all over the world.

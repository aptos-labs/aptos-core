---
title: "Node Health Checker FAQ"
slug: "node-health-checker-faq"
---

# Node Health Checker FAQ

The Aptos Node Health Checker (NHC) service can be used to check the health of your node(s). See [Node Health Checker](/nodes/node-health-checker/index) for full documentation on the NHC.

The purpose of this FAQ is to help you understand why your node did not pass a particular health check when you ran NHC for it. If you didn't find the information you wanted in this FAQ, [open an issue](https://github.com/aptos-labs/aptos-core/issues/new/choose), or [open a PR](https://github.com/aptos-labs/aptos-core/pulls) and add the information yourself.

## How does the latency evaluator work?

You are likely here because you were given an NHC evaluation result like this:

```
Average latency too high: The average latency was 1216ms, which is higher than the maximum allowed latency of 1000ms.
```

While the NHC reports 1216ms above, when you `ping` you might see a latency like 600ms. This difference is because when you `ping` an IP, the result you see is a single round trip (where the latency is the round trip time (RTT)). On the other hand, the NHC latency test will a request to the API running on your node. In effect, this means that the NHC will time 2 round trips, because it does the following:

1. SYN
2. SYNACK
3. ACK + Send HTTP request
4. Receive HTTP response

i.e., the NHC must do a TCP handshake (one round trip) and then make an HTTP request (second round trip).

The reason the NHC uses the latency evaluator is to ensure that we can maintain good network performance. In particular, if the latency to your node is too high, it will result in a low TPS and high time to finality, both of which are very important to running a highly performant L1 blockchain. **If you receive this error, you will need to try and improve the latency to your node. We have set high thresholds on this value with the understanding that nodes will be running all over the world**.

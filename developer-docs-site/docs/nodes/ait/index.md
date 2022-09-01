---
title: "AIT-3"
slug: "ait-3"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# AIT-3

<p class="card-section-h2">Welcome to AIT-3</p>

The Aptos Incentivized Testnet-3 (AIT-3) is a rewards program for any Aptos community member. All you need is be interested, willing and capable of running an Aptos node, validator node and public fullnode, and to test the new features.

## (New) Key AIT-3 dates for public fullnodes

:::tip **Key AIT-3 dates for public fullnodes:**
_All dates and times shown are for Pacific Time, year 2022._

- **September 1:** Registration for public fullnode starts. Public fullnode and identity verification begins.
- **September 4:** Registration for public fullnode ends.
- **September 5:** Notification of the public fullnode selection results sent out.
- **September 6:** Healthcheck for the public fullnodes starts.
:::

### Goals for public fullnodes in AIT-3

- Onboard 10k+ public fullnodes (PFN) onto AIT-3 to stress test the Aptos network.
- Validate the fullnode documentation and tooling.

:::tip Validator node / validator fullnode / public fullnode
A validator fullnode represents a security barrier, protecting the validator nodes at the core of the Aptos network from any security threats from public fullnodes at the edge of the Aptos network. 

Logically, a validator fullnode is between a public fullnode and a validator node. A public fullnode will only connect to a validator fullnode or to another public fullnode. Validator fullnodes connect directly to validator nodes and offer scalability alongside DDoS mitigation. Public fullnodes connect to validator fullnodes (or other public fullnodes) to gain low-latency access to the Aptos network. 

See [Node network topology](/concepts/basics-node-networks-sync.md) and the Medium post [The Evolution of State Sync: The path to 100k+ transactions per second with sub-second latency at Aptos](https://medium.com/aptoslabs/the-evolution-of-state-sync-the-path-to-100k-transactions-per-second-with-sub-second-latency-at-52e25a2c6f10).
:::

### Success criteria

- The functionality and performance of a validator node or of a validator fullnode (VFN) is not expected to be impacted during this test.
- The public fullnode (PFN) is expected to sync, catchup and perform.
- Rate limiting should work as expected.
- We look to the community to support each other using the Aptos documentation.

### End-to-end AIT-3 flow for public fullnodes

1. Aptos team announces the plan to onboard 10k+ public fullnodes.
2. The Aptos community platform is opened for those who already passed KYC. These members should provide their updated fullnode information. **Only those who have already registered for AIT3 are allowed this step.**
3. The public fullnode operator should reconfigure their public fullnode to ensure that the public fullnode starts with the static identity. This is to ensure that they can provide public key information for validation later by the Aptos team.

  :::tip Starting a node with static identity
  Follow the steps described in [Network Identity For FullNode](/nodes/full-node/network-identity-fullnode) to start a node with static identity.
  :::

4. The Aptos validator fullnodes have connection limits. Hence, at some point the node operators should help each other to connect by advertising their address, and let others use them as seed. 

  :::tip Allowing other FullNodes to connect
  Follow the steps described in [Allowing other FullNodes to connect](/nodes/full-node/network-identity-fullnode/#allowing-other-fullnodes-to-connect) to allow others to connect to the Aptos network through your node.
  :::

5. Use the following **Discord** channels for your public fullnode discussions and help:
    - `#ait3-fullnode-support` for participants to communicate internally and support each other.
    - `#advertise-ait3-fullnode` channel to share identity information.

6. The node health checker [(NHC)](/nodes/node-health-checker) will health check the selected public fullnodes, and the pass or fail decision will be made on the NHC results and the [telemetry](/reference/telemetry) data submitted by the public fullnode. 

7. The Aptos AIT-3 team will run load testing by submitting transactions through the selected public fullnodes.

### Rewards

- Successful public fullnode operators will receive 200 Aptos tokens.

### Rewards criteria

The participating public fullnodes:

- Must have a static identity.
- Meet [node liveness](/nodes/ait/node-liveness-criteria) as defined by metrics push data ≥ 95%.
- Pass the node health checker [NHC](/nodes/node-health-checker) more than 90% of the time.
- Perform any node operations requested by the Aptos team within 24 hours of notice posted in the Aptos Discord channel.

### Running public fullnode for AIT-3

<div class="docs-card-container">
<div class="row row-cols-1 row-cols-md-1 g-4">

   <div class="col">
    <div class="card h-100" >
    <div class="card-body d-flex flex-column" >
    <h2 class="card-title">Public Fullnode for AIT-3</h2>
    <p class="card-text"><a href="/nodes/full-node/fullnode-for-devnet" class="card-link">Guides for running a public fullnode using Google Cloud or Docker or Aptos source.</a></p>
</div>
</div>
</div>
  
</div>
</div>

## Key AIT-3 dates for validators

:::tip **Key AIT-3 dates for validators:**
_All dates and times shown are for Pacific Time, year 2022._

- **August 19:** Registration starts. Validator node and identity verification begins.
- **August 25:** Registration ends.
- **August 29:** Notification of the selection results for validators sent out.
- **August 30:** AIT-3 becomes live.
- **September 9:** AIT-3 concludes.

:::

<div class="docs-card-container">
<div class="row row-cols-1 row-cols-md-2 g-4">
  <div class="col">
    <div class="card h-100">
    <h3 class="card-header">1. See</h3>
      <div class="card-body d-flex flex-column">
        <a href="/nodes/ait/whats-new-in-ait3" class="card-title card-link"> <h2>What's New in AIT-3</h2></a>
        <p class="card-text">See the new features that are up for testing by the AIT-3 participants. </p>
      </div>
    </div>
  </div>
  <div class="col" >
    <div class="card h-100">
     <h3 class="card-header">2. Participate</h3>
      <div class="card-body d-flex flex-column">
      <a href="/nodes/ait/steps-in-ait3" class="card-title card-link stretched-link"> <h2>Steps in AIT-3</h2></a>
        <p class="card-text">Start participating. Read the summary flowchart and the detailed steps of the AIT-3 program.</p>     
      </div>
    </div>
  </div>  
</div>
</div>


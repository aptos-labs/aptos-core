---
title: "Node Inspection Service"
slug: "node-inspection-service"
---

# Node Inspection Service 

Aptos nodes collect metrics and system information while running. These provide a way to track,
monitor and inspect the health and performance of the node dynamically, at runtime. Node metrics and system
information can be queried or exported via an inspection service that runs on each node.

## Examining node metrics

If you'd like to examine the metrics of your node (e.g., validator or fullnode), start running a
node and visit the inspection service locally, e.g., visit this url in your browser:

```
http://localhost:9101/metrics
```

This will display the values of all the metrics and counters of your node at the time you queried it.
To see updates to these values, simply refresh the page.

Likewise, if you wish to view the metrics in `json` format, visit the following url:

```
http://localhost:9101/json_metrics
```

:::tip Inspection service configuration
The inspection service should run on all nodes by default, at port `9101`. See additional configuration
details below.
:::

## Configuring the inspection service

You can configure the node inspection service in various different ways. For example, to change
the port the inspection service listens on (e.g., to `1000`), add the following to your node
configuration file:

```yaml
 inspection_service:
     port: 1000
```

## Examining system information

The inspection service also provides a way to examine the configuration of your node
at runtime (i.e., the configuration settings that your node started with). To enable
this feature, add the following to your node configuration file:

```yaml
 inspection_service:
   expose_configuration: true
```

And visit the configuration url:

```
http://localhost:9101/configuration
```

:::caution Proceed with caution
By default, the configuration endpoint is disabled as it may expose potentially sensitive
information about the configuration of your node, e.g., file paths and directories. We only
recommend enabling this endpoint if the inspection service is not publicly accessible.
:::`

Likewise, the inspection service also provides a way to examine the system information of your node
at runtime (i.e., build and hardware information). Simply visit the following url:

```
http://localhost:9101/system_information
```

If you'd like to disable this endpoint, add the following to your node configuration file:

```yaml
 inspection_service:
   expose_system_information: false
``` 

:::tip System information accuracy
The system information displayed here is not guaranteed to be 100% accurate due to limitations
in the way this information is collected. As a result, we recommend not worrying about any
inaccuracies and treating the information as an estimate.
:::`

## Understanding node metrics

When you visit the metrics endpoint, you will notice that there are a large number of metrics
and counters being produced by your node. Most of these metrics and counters are useful only for
blockchain development and diagnosing hard to find issues. As a result, we recommend that node
operators ignore most metrics and only pay attention to the key metrics presented below:

:::caution Metric instability
As Aptos continues to grow and develop the blockchain software, many metrics will come and go.
As a result, we only recommend relying on the presence of the metrics explicitly mentioned below.
All other metrics should be considered unstable and may be changed/removed without warning.
:::

### Consensus

If you are running a validator node, the following consensus metrics are important:
1. `aptos_consensus_proposals_count`: This metric counts the number of times the node sent a block
proposal to the network. The count will only increase when the validator is chosen to be a proposer,
which depends on the node's stake and leader election reputation. You should expect this metric to
increase at least once per hour.
2. `aptos_consensus_last_committed_round`: This metric counts the last committed round of the node.
During consensus, we expect this value to increase once per consensus round, which should be multiple
times per second. If this does not happen, it is likely the node is not participating in consensus.
3. `aptos_consensus_timeout_count`: Counts the number of times the node locally timed out while trying
to participate in consensus. If this counter increases, it is likely the node is not participating 
in consensus and may be having issues, e.g., network difficulties.
4. `aptos_state_sync_executing_component_counters{label="consensus"`: This counter increases
a few times per second as long as the node is participating in consensus. When this counter stops
increasing, it means the node is not participating in consensus, and has likely fallen back to state
synchronization (e.g., because it fell behind the rest of the validators and needs to catch up).

### State Sync

If you are running a fullnode (or a validator that still needs to synchronize to the latest blockchain
state), the following state sync metrics are important:
1. `aptos_state_sync_version{type="synced"}`: This metric displays the current synced version of the node,
i.e., the number of transactions the node has processed. If this  metric stops increasing, it means the
node is not syncing. Likewise, if this metric doesn't increase faster than the rate at which new transactions
are committed to the blockchain, it means the node is unlikely to get and stay up-to-date with the rest of
the network. Note: if you've selected to use [fast sync](../../guides/state-sync#fast-syncing),
this metric won't increase until all states have been downloaded, which may take some time. See (3) below.
2. `aptos_data_client_highest_advertised_data{data_type="transactions"}`: This metric displays the highest
version synced and advertised by the peers that your node is connected to. As a result, when this metric is
higher than `aptos_state_sync_version{type="synced"}` (above) it means your node can see new blockchain data and
will sync the data from its peers.
3. `aptos_state_sync_version{type="synced_states"}`: This metric counts the number of states that have been
downloaded while a node is [fast syncing](../../guides/state-sync#fast-syncing). If this metric doesn't increase,
and `aptos_state_sync_version{type="synced"}` doesn't increase (from above), it means the node is not syncing
at all and an issue has likely occurred.
4. `aptos_state_sync_bootstrapper_errors` and `aptos_state_sync_continuous_syncer_errors`: If your node is
facing issues syncing (or is seeing transient failures), these metrics will increase each time an error occurs.
The `error_label` inside these metrics will display the error type.

:::tip
Compare the synced version shown by `aptos_state_sync_version{type="synced"}` with the highest version shown on the
[Aptos explorer page](https://explorer.aptoslabs.com/?network=mainnet), to see how far behind the latest blockchain
version your node is. Remember to select the correct network that your node is syncing to (e.g., `mainnet`).
:::

### Networking

The following network metrics are important, for both validators and fullnodes:
1. `aptos_connections{direction="inbound"` and `aptos_connections{direction="outbound"`: These metrics count
the number of peers your node is connected to, as well as the direction of the network connection. An `inbound`
connection means that a peer (e.g., another fullnode) has connected to you. An `outbound` connection means that
your node has connected to another node (e.g., connected to a validator fullnode).
   1. If your node is a validator, the sum of both `inbound` and `outbound` connections should be equal to the
   number of other validators in the network. Note that only the sum of these connections matter. If all connections
   are `inbound`, or all are `outbound`, this doesn't matter.
   2. If your node is a fullnode, the number of `outbound` connections should be `> 0`. This will ensure your node is
   able to synchronize. Note that the number of `inbound` connections only matters if you want to act as a seed in
   the network and allow other nodes to connect to you (as discussed in the
   [Fullnode Network Connections](../../nodes/full-node/fullnode-network-connections#allowing-fullnodes-to-connect-to-your-node) document).


### Mempool

The following mempool metrics are important:
1. `core_mempool_index_size{index="system_ttl"`: This metric displays the number of transactions currently sitting in
the mempool of the node and waiting to be committed to the blockchain:
   1. If your node is a fullnode, it's highly unlikely that this metric will be `> 0`, unless transactions are actively
   being sent to your node via the REST API and/or other fullnodes that have connected to you. Most fullnode operators
   should ignore this metric.
   2. If your node is a validator, you can use this metric to see if transactions from your node's mempool are being
   included in the blockchain (e.g., if the count decreases). Likewise, if this metric only increases, it means
   that either: (i) your node is unable to forward transactions to other validators to be included in the blockchain; or
   (ii) that the entire blockchain is under heavy load and may soon become congested.

### REST API

The following REST API metrics are important:
1. `aptos_api_requests_count{method="GET"` and `aptos_api_requests_count{method="POST"`: These metrics counts
the number of REST API `GET` and `POST` requests that have been received via the node's REST API. This
allows you to monitor and track the amount of REST API traffic on your node. You can also further use the
`operation_id` in the metric to monitor the type of operation the requests are performing.

2. `aptos_api_response_status_count`: This metric counts the number of response types that were sent for
the REST API. For example, `aptos_api_response_status_count{status="200"}` counts the number of requests
that were successfully handled with a `200` response code. You can use this metric to track the success and
failure rate of the REST API traffic.

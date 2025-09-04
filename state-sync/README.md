---
id: state sync
title: State Sync
custom_edit_url: https://github.com/velor-chain/velor-core/edit/main/state-sync/README.md
---

# State Synchronization (State Sync)

State sync is a component that runs within each Velor node and is responsible
for synchronizing the node to the latest blockchain state. It is required by
both validators and fullnodes to ensure that they do not fall behind the rest
of the network. To achieve this, state sync identifies and fetches new
blockchain data from peers, validates the data and persists it to local
storage.

To read more about state sync and node configurations, see the [state sync developer documentation](https://velor.dev/guides/state-sync/).

## State sync architecture

State sync is broken down into 4 sub-components, each with a specific purpose:

1. **Driver**: The driver “drives” the synchronization progress of the node.
It is responsible for verifying all data that it receives from peers. Data
is forwarded from peers via the data streaming service. After data
verification, the driver persists the data to storage.
2. **Data Streaming Service**: The streaming service creates data streams for
clients (one of which is the state sync driver). It allows the client to stream
new data chunks from peers, without having to worry about which peers have the
data or how to manage data requests. For example, the client can request all
transactions since version `5` and the data streaming service will provide
this.
3. **Velor Data Client**: The data client is responsible for handling data
requests from the data streaming service. For the data streaming service to
stream all transactions, it must make multiple requests (each request for a
batch of transactions) and send those requests to peers (e.g., transactions
`1→5`, `6→10`, `11→15`, and so on). The data client takes the request,
identifies which peer can handle the request and sends the request to them.
4. **Storage Service**: The storage service is a simple storage API offered by
each node which allows peers to fetch data. For example, the data client on
peer `X` can send the data request to the storage service on peer `Y` to fetch
a batch of transactions.

### Code structure

The state sync code structure matches the architecture outlined above:
- **Driver:** [https://github.com/velor-chain/velor-core/tree/main/state-sync/state-sync-driver](https://github.com/velor-chain/velor-core/tree/main/state-sync/state-sync-driver)
- **Data Streaming Service:** [https://github.com/velor-chain/velor-core/tree/main/state-sync/data-streaming-service](https://github.com/velor-chain/velor-core/tree/main/state-sync/data-streaming-service)
- **Velor Data Client**: [https://github.com/velor-chain/velor-core/tree/main/state-sync/velor-data-client](https://github.com/velor-chain/velor-core/tree/main/state-sync/velor-data-client)
- **Storage Service:** [https://github.com/velor-chain/velor-core/tree/main/state-sync/storage-service](https://github.com/velor-chain/velor-core/tree/main/state-sync/storage-service)

In addition, there is also a directory containing the code for
**inter-component** communication: [https://github.com/velor-chain/velor-core/tree/main/state-sync/inter-component](https://github.com/velor-chain/velor-core/tree/main/state-sync/inter-component).
This is required so that:
   - State sync can handle notifications from consensus (e.g., to catch up after falling behind)
   - State sync can notify mempool when transactions are committed (i.e., so they can be removed from mempool)
   - State sync can update the event subscription service to notify listeners (e.g., other system components for reconfiguration events)


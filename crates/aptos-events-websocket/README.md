# Aptos Events Websocket on Fullnode

This opens an events websocket endpoint on the fullnode.

## Local testing
### 1) Run the fullnode

#### Against an existing network

Follow instructions on how to run a fullnode against an existing network.
* Get genesis, waypoint, and fullnode.yaml
* Add following to fullnode.yaml
  * ```
    storage:
      enable_indexer: true

    indexer_grpc:
      enabled: true
      address: 0.0.0.0:50051
      processor_task_count: 10
      processor_batch_size: 100
      output_batch_size: 100```
* Run fullnode `cargo run -p aptos-node --release -- -f ./fullnode.yaml`

### 2) Test with GCURL
* Install grpcurl (https://github.com/fullstorydev/grpcurl#installation)
* From the aptos-core (base folder), test with grpcurl: `grpcurl  -max-msg-sz 10000000 -d '{ "starting_version": 0 }' -import-path crates/aptos-protos/proto -proto aptos/internal/fullnode/v1/fullnode_data.proto  -plaintext 127.0.0.1:50051 aptos.internal.fullnode.v1.FullnodeData/GetTransactionsFromNode`

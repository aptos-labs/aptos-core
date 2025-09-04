# Velor Indexer GRPC on Fullnode

This opens a GRPC endpoint on the indexer. A client (e.g. worker) connects to the endpoint and makes a request. The GRPC endpoint would maintain a stream and sends transactions back to the client on a batch basis. Note that transactions within a batch may be out of order. 

TBD architecture diagram. Also link to dev docs

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
* Run fullnode `cargo run -p velor-node --release -- -f ./fullnode.yaml`

### 2) Test with GCURL
* Install grpcurl (https://github.com/fullstorydev/grpcurl#installation)
* From the velor-core (base folder), test with grpcurl: `grpcurl  -max-msg-sz 10000000 -d '{ "starting_version": 0 }' -import-path crates/velor-protos/proto -proto velor/internal/fullnode/v1/fullnode_data.proto  -plaintext 127.0.0.1:50051 velor.internal.fullnode.v1.FullnodeData/GetTransactionsFromNode`

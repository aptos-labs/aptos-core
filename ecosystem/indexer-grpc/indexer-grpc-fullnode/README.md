# Aptos Indexer GRPC on Fullnode

This opens a GRPC endpoint on the indexer. A client (e.g. worker) connects to the endpoint and makes a request. The GRPC endpoint would maintain a stream and sends transactions back to the client on a batch basis. Note that transactions within a batch may be out of order. 

TBD architecture diagram. Also link to dev docs

## Local testing
### 1) Run the fullnode
* Follow instructions on how to get genesis, waypoint, and fullnode.yaml. 
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
* From the aptos-core (base folder), test with grpcurl
`grpcurl  -max-msg-sz 10000000 -d '{ "starting_version": 0, "chain_id": 1}' -import-path crates/aptos-protos/proto -proto aptos/datastream/v1/datastream.proto  -plaintext 127.0.0.1:50051 aptos.datastream.v1.IndexerStream/RawDatastream`

### 3) Test with rust client
TBD

## Deployment
### Docker


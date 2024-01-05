# Aptos Indexer GRPC Table Info on Fullnode

This indexes and parses table info mapping on fullnode.

## Local testing
### 1) Run the fullnode

#### Against an existing network

Follow instructions on how to run a fullnode against an existing network.
* Get genesis, waypoint, and fullnode.yaml
* Add following to fullnode.yaml
  * ```
    indexer_table_info:
      enabled: true
      parser_task_count: 10
      parser_batch_size: 1000

* Run fullnode `cargo run -p aptos-node --release -- -f ./fullnode.yaml`

# Aptos Indexer GRPC

* Indexer grpc tails the transactions(from starting version) on node and streams out the result via grpc.

* Example invocation:

```bash
cargo run -p aptos-node --release -- -f <some_path>/fullnode.yaml
```

* Example fullnode.yaml modification
    ```
    storage:
        enable_indexer: true
    # This is to avoid the node being pruned
    storage_pruner_config:
        ledger_pruner_config:
        enable: false
    indexer_grpc:
        enabled: true
        port: 50051
    ```

## Request(`aptos.datastream.v1.RawDatastreamRequest`) 
* `starting_version`
  * The starting version of current stream. 
* `processor_task_count`
  * The number of tasks to fetch data from the node.
* `processor_batch_size`
  * The number of transactions each task to fetch from the node.
* `output_batch_size`
  * The upper liimit for number of transactions in each response.
  * Ideally, if there are enough transactions to fetch, e.g., far from head,
  ```
    number_of_response * number_of_output_batch_size ==
        processor_task_count * processor_batch_size
  ```
* `chain_id`
  * The target chain id; this is for verification purpose.

## Response(`aptos.datastream.v1.RawDatastreamResponse`)

The streaming response consists:

* One `INIT` response
* Streaming batches and one batch of responses consists
  *  One or more(`number_of_response`) `Data` response; these responses can be out of order.
     *  Transactions within one response are ordered by versions.
  *  One `Batch_END` response with end_version for current batch.  
     *  This version is the starting version for the next batch.
# Indexer GRPC file store

File store backfiller.

# Why you need a backfiller?

In rare case, upstream changes that don't reflect in proto schema require backfill.
To mitigate the issue, you need a backfiller to reply transactions against the new proto
schema so that the changes can surface up. 

## Prerequisite

* A running archival fullnode with `index` enabled. You can check more from [here](https://velor.dev/en/network/nodes/configure/state-sync#archival-pfns) and [fullnode config](https://github.com/velor-chain/velor-core/tree/main/ecosystem/indexer-grpc/indexer-grpc-fullnode).


## How to run it.

Example of config: 

```
health_check_port: 8081
    server_config:
      fullnode_grpc_address: >-
        http://your.node.address:50051
      file_store_config:
        file_store_type: GcsFileStore
        gcs_file_store_bucket_name: your-gcs-bucket-name
        gcs_file_store_service_account_key_path: /secrets/your-service-account-key
        enable_compression: true
      progress_file_path: /path-to-file/progress_tracker.json
      chain_id: 2
      starting_version: 0
      transactions_count: 100000000
      enable_cache_compression: true
```

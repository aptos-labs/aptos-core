# Velor Indexer GRPC Table Info on Fullnode

This indexes and parses table info mapping on fullnode.

## Local testing
### 1) Run the fullnode

#### Against an existing network

Follow instructions on how to run a fullnode against an existing network.
* Get genesis, waypoint, and fullnode.yaml
* Add following to fullnode.yaml to backup your table info db.
```
  indexer_table_info:
    parser_task_count: 10
    parser_batch_size: 100
    table_info_service_mode:
        Backup:
            your-bucket-name
  ```

* Similarly, if you don't want to use the async db only
```
  indexer_table_info:
      ...
      table_info_service_mode:
          IndexingOnly
```

* If you want to disable completely, 
```
  indexer_table_info:
      ...
      table_info_service_mode:
          Disabled
```
* To use the restore service, 

```
  indexer_table_info:
    ...
    table_info_service_mode:
        Restore:
            your-bucket-name
```

* Run fullnode `cargo run -p velor-node --release -- -f ./fullnode.yaml`

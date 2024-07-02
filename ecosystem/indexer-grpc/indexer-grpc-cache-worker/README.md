# Indexer GRPC cache worker

Cache worker fetches data from fullnode GRPC and push data to Cache. 

## How to run it.

* service account json with `read` access to bucket `${file_store_bucket_name}`, e.g., `xxx.json`.
  
* `SERVICE_ACCOUNT` env var pointing to service account json file.

* Run it:  `cargo run --release -- -c config.yaml`

* Yaml Example 
```yaml
health_check_port: 8083
server_config:
    fullnode_grpc_address: 0.0.0.0:50052
    file_store_config:
      file_store_type: GcsFileStore
      gcs_file_store_bucket_name: indexer-grpc-file-store-bucketname
    redis_main_instance_address: 127.0.0.1:6379
```

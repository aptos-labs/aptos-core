# Indexer GRPC cache worker

Cache worker fetches data from fullnode GRPC and push data to Cache. 

## How to run it.

* service account json with `read` access to bucket `${file_store_bucket_name}`, e.g., `xxx.json`.
  
* `SERVICE_ACCOUNT` env var pointing to service account json file.

* Run it:  `cargo run --release -- -c config.yaml`

* Yaml Example 
```yaml
fullnode_grpc_address: 127.0.0.1:50051
redis_address: larry.macos.network:6379
file_store_bucket_name: indexer-grpc-file-store-testnet 
health_check_port: 8081
```
# Indexer GRPC data service

Indexer GRPC data service fetches data from both cache and file store.

## How to run it.

* service account json with `read` access to bucket `${file_store_bucket_name}`, e.g., `xxx.json`.
  
* `SERVICE_ACCOUNT` env var pointing to service account json file.

* Run it:  `cargo run --release -- -c config.yaml`

* Yaml Example 

```yaml
data_service_grpc_listen_address: 0.0.0.0:50052
redis_address: 127.0.0.1:6379
file_store_bucket_name: indexer-grpc-file-store-testnet 
health_check_port: 8081
```
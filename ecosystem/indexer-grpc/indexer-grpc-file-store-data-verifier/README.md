# Data Verifier
* This is to verify that data stored in GCS and fetched from fullnode are identical.

## How to Run it

* service account json with `write` access to bucket `${file_store_bucket_name}`, e.g., `xxx.json`.
  
* `SERVICE_ACCOUNT` env var pointing to service account json file.

* Run it:  `cargo run --release -- -c config.yaml`

* Yaml Example 
```yaml
fullnode_grpc_address: 127.0.0.1:50051
redis_address: 127.0.0.1:6379
file_store_bucket_name: indexer-grpc-file-store-testnet 
health_check_port: 8083
```
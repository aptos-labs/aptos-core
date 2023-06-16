# Indexer GRPC data service

Indexer GRPC data service fetches data from both cache and file store.

## How to run it.

* service account json with `read` access to bucket `${file_store_bucket_name}`, e.g., `xxx.json`.

* `SERVICE_ACCOUNT` env var pointing to service account json file.

* Run it:  `cargo run --release -- -c config.yaml`

* Yaml Example

```yaml
health_check_port: 8083
server_config:
    data_service_grpc_listen_address: 0.0.0.0:50052
    whitelisted_auth_tokens: 
      - "token1"
      - "token2"
    file_store_config:
      file_store_type: GcsFileStore
      gcs_file_store_bucket_name: indexer-grpc-file-store-bucketname
    data_service_grpc_tls_config:
      cert_path: /path/to/cert.cert
      key_path: /path/to/key.pem
    redis_read_replica_address: 127.0.0.1:6379
```

## How to use grpc web UI
Install the tool, for example on Mac:
```
brew install grpcui
```

Run the server:
```
cargo run --release -- -c config.yaml
```

Open the web UI:
```
grpcui -plaintext 127.0.0.1:50052
```

The port used here should match the port used in `data_service_grpc_listen_address` in the config file.

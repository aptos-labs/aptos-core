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
whitelisted_auth_tokens: ["PUT YOUR TOKEN 1", "PUT YOUR TOKEN 2"]
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

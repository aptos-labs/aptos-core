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
    whitelisted_auth_tokens: 
      - "token1"
      - "token2"
    file_store_config:
      file_store_type: GcsFileStore
      gcs_file_store_bucket_name: indexer-grpc-file-store-bucketname
    data_service_grpc_tls_config:
      data_service_grpc_listen_address: 0.0.0.0:50052
      cert_path: /path/to/cert.cert
      key_path: /path/to/key.pem
    data_service_grpc_non_tls_config:
      data_service_grpc_listen_address: 0.0.0.0:50051
    redis_read_replica_address: 127.0.0.1:6379
```

### Config Explanation

* `whitelisted_auth_tokens`: list of tokens that are allowed to consume the endpoint.
* `file_store_config`: GCS to archive the transactions
* `data_service_grpc_tls_config`: TLS endpoint exposed
  * GPRC endpoint with TLS, i.e., https. It's ok to expose tls endpoint only.
  * We introduce it here(in a non mutual-exclusive way) to avoid potential compatibility issue for clients. 
* `data_service_grpc_non_tls_config`: Non-TLS endpoint exposed
  * GRPC endpoint without TLS, i.e., http. It's ok to expose non-tls only.

### HTTP2-ping-based liveness check

Long-live connections are prune to network errors. We introduce HTTP2 ping check to actively detect if 
connection is broken.

* `HTTP2_PING_INTERVAL_DURATION`: const value for http2 ping interval; default to 60s.
* `HTTP2_PING_TIMEOUT_DURATION`: const value for http2 ping timeout; default to 10s. 

Note: this requires http2 ping support, e.g., AWS/ALB may not work(https://stackoverflow.com/questions/66818645/http2-ping-frames-over-aws-alb-grpc-keepalive-ping).

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

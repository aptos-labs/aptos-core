# Aptos Indexer GRPC GCS migration tool

This tool is to migrate the GCS from one format to another.

## How to run this tool

* Example Config

```yaml
health_check_port: 8081
server_config:
  legacy_file_store_config:
    file_store_type: GcsFileStore
    gcs_file_store_bucket_name: legacy_bucket_name
    gcs_file_store_service_account_key_path: /path/to/servce/account/key
  new_file_store_config:
    file_store_type: GcsFileStore
    gcs_file_store_bucket_name: new_bucket_name
    gcs_file_store_service_account_key_path: /path/to/servce/account/key
  # chain id for verification.
  chain_id: 1
```

* To run `cargo run -- -c config.yaml`.
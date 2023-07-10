# Indexer GRPC Post-processor(background workers)

Indexer GRPC Post-processor(background workers) is an optional component for Indexer GRPC system, which is used for data quality check, monitoring purpose.  

## Usage
```yaml
health_check_port: 8088
server_config:
    file_storage_verifier:
        chain_id: 61
        file_store_config:
            file_store_type: GcsFileStore
            gcs_file_store_bucket_name: bucket_name_for_file_store
            gcs_file_store_service_account_key_path: /path/to/service_account.json
    pfn_checker_config:
        public_fullnode_addresses:
            - http://fullnode.1.address/v1
            - http://fullnode.2.address/v1
            - http://fullnode.3.address/v1
        indexer_grpc_address: IP:PORT
        indexer_grpc_auth_token: AUTH_TOKEN
```
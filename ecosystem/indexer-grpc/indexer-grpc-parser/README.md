# Aptos Indexer GRPC Parser

## Example Yaml file

```yaml
indexer_grpc_address: "ADDRESS:PORT" # port typically 50051
postgres_connection_string: "POSTGRES URL" # e.g. postgres://postgres@localhost/indexer_v3
number_concurrent_processing_tasks: 20
processor_name: default_processor
health_check_port: 8089
indexer_grpc_auth_token: "AUTH TOKEN"
```

* Command to run the parser
  * `cargo run -- -c ~/configs/parser.yaml`

# Aptos Indexer GRPC Parser

## Example Yaml file

```yaml
indexer_grpc_address: http://192.168.1.123:50051
postgres_connection_string: postgres://postgres@localhost/indexer_v3
number_concurrent_processing_tasks: 20
processor_name: default_processor
health_check_port: 8089  
```

* Command to run the parser
  * `cargo run -- -c ~/configs/parser.yaml`

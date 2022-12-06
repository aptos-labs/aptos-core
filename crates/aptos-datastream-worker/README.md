# Aptos Datastream Worker

Aptos Datastream worker requests streaming data from Indexer gRPC and push to Redis.

## Running the worker
```
RUST_BACKTRACE=full WORKER_CONFIG_PATH=/YOUR_PATH_TO_YAML_FILE/datastream_worker.yaml cargo run
```

### YAML Exmaple
```
indexer_address: "localhost"  
indexer_port: 50051  
redis_address: 127.0.0.1  
redis_port: 6379  
starting_version: 1  
chain_id: 2
```
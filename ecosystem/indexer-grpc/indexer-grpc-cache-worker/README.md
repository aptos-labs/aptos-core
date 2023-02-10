# Indexer GRPC cache worker

Cache worker fetches data from fullnode GRPC and push data to Cache. 

## How to run it.

* A yaml file for the worker.

```yaml
indexer_address: 127.0.0.1:50051
redis_address: 127.0.0.1:6379
starting_version: 0
chain_id: 43
```


* Set the `WORKER_CONFIG_PATH` ENV varaible to your yaml fille, and run your cache worker at current folder,
    `cargo run --release -- --config-path=worker.yaml`
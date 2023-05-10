# Indexer GRPC

Indexer GRPC is an infrastructure for efficiently serving on-chain data for indexing with low-latency.


## Setup 

### Requirement

* Redis with at least 50 GB of memory.
* Google Cloud Storage with one bucket and one service account JSON, which should be assigned as `Object Owner` and `Bucket Owner`.
  * `Object Owner` is to raed and write to each file.
  * `Bucket Owner` is to verify the bucket existence.

### How to Start the Infrastructure

* Start the full node and cache worker (for more information, refer to `indexer-grpc-cache-worker`)
  * Note: : the cache worker will exit after 1 minute since the file store is not ready. Please restart it.
* Start the file store worker (for more information, refer to `indexer-grpc-file-store`).
* Start the data service (for more information, refer to `indexer-grpc-data-service`).
# Indexer GRPC data service

Indexer GRPC data service fetches data from both cache and file store.

## How to run it.

* service account json with write access to bucket `{FILE_STORE_BUCKET_NAME}-{CHAIN_NAME}`, e.g., with name `xxx.json`.
  
* Command-line to run:

```bash
SERVICE_ACCOUNT=xxx.json \
REDIS_ADDRESS=127.0.0.1 \
CHAIN_NAME=devnet \
GRPC_ADDRESS=0.0.0.0:50052 \
FILE_STORE_BUCKET_NAME=indexer-grpc-file-store \
FILE_STORE_BLOB_FOLDER_NAME=blobs \
HEALTH_CHECK_PORT=8083 \
cargo run --release
```
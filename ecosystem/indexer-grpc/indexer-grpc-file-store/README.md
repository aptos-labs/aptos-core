# Indexer GRPC file store

File store fetches data from cache and stores in Google Cloud Storage.

## How to run it.

* A service account json with write access to GCS.
  * To bootstrap, please upload `metadata.json` to your bucket(`$FILE_STORE_BUCKET_NAME` e.g., `indexer-grpc-file-store`):
  ```json   
    {
        "chain_id": 43,
        "blob_size": 1000,
        "version": 0
    }
  ```
  * `chain_id` is the chain to process, immutable.
  * `blob_size` is the number of transactions in each blob, immutable.
  * `version` is the current version of transaction to process.

* A Redis cache running at `$REDIS_ADDRESS`, e.g., `127.0.0.1:6379`
* Example command to run:

```bash
SERVICE_ACCOUNT=YOUR_JSON.json \
REDIS_ADDRESS=127.0.0.1:6379 \
CHAIN_ID=43 \
FILE_STORE_BUCKET_NAME=indexer-grpc-file-store \
FILE_STORE_BLOB_FOLDER_NAME=blobs cargo run --release
```

* Your bucket looks like:

```bash
indexer-grpc-file-store/
    blobs/
        0_999.json
        1000_1999.json
        ...
    metadata.json
```

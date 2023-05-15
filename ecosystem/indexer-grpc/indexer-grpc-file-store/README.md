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
fullnode_grpc_address: 127.0.0.1:50051
redis_address: 127.0.0.1:6379
health_check_port: 8083
file_store:
  file_store_type: GcsFileStore
  gcs_file_store_bucket_name: indexer-grpc-file-store-testnet 
```

* Your bucket looks like:

```bash
indexer-grpc-file-store-testnet/
    files/
        0.json
        1000.json
        ...
    metadata.json
```

## [TEST ONLY] Run it with a local filestore

For developing and testing locally, it might be easier to use a local filestore.

Create a local directory to store the filestore: `mkdir test_indexer_grpc_filestore`

Then in your config:
```
...
file_store:
  file_store_type: LocalFileStore
  local_file_store_path: test_indexer_grpc_filestore
```

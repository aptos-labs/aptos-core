# Indexer Transaction Generator

This tool is to generate transactions for testing purpose.

## Usage

`cargo run -- --config example.yaml --output-folder /your_path_to_store_transactions/`

### Config

```YAML
import_config:
  testnet:
    # Transaction Stream endpoint addresss.
    transaction_stream_endpoint: https://grpc.testnet.aptoslabs.com:443
    # (Optional) The key to use with developers.aptoslabs.com
    api_key: YOUR_KEY_HERE
    # A map from versions to dump and their output names.
    versions_to_import:
      123: testnet_v1.json
```

# Indexer Transaction Generator

This tool is to generate transactions for testing purpose.

## Usage

`cargo run -- --config example.yaml --output-folder /your_path_to_store_transactions/`

### Config

```YAML
# Config to import transactions onchain.
import_config:
  testnet:
    # Transaction Stream endpoint addresss.
    transaction_stream_endpoint: https://grpc.testnet.aptoslabs.com:443
    # (Optional) The key to use with developers.aptoslabs.com
    api_key: YOUR_KEY_HERE
    # A map from versions to dump and their output names.
    versions_to_import:
      123: testnet_v1.json
# Config to generate the transactions via localnode.
script_transaction_generator_config:
  scripted_transactions:
    # Steps can be shared between runs.
    - steps:
        - script_path: /path/to/your_move_script
          output_name: random_script.json
```

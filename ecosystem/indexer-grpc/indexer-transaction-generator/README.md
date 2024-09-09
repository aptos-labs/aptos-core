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
  runs:
    - transactions:
      - output_name: transfer_from_a_to_b
        script_path: path/to/script1
        fund_address: address_a
      - output_name: transfer_from_b_to_c
        script_path: path/to/script2
    - transactions:
      # Note: we've generated transactions for script1, we don't need the name anymore.
      - script_path: path/to/script1
        fund_address: address_a
      - output_name: burn
        script_path: path/to/script3


```

### Recommended file structure
```
your_testing_folder/
├─ config.yaml
├─ move_files/
│  ├─ your_first_move_script/
│  │  ├─ .aptos/
│  │  ├─ Move.toml
│  │  ├─ sources/
│  │  │  ├─ main.move
│  ├─ your_second_move_script/
...

```
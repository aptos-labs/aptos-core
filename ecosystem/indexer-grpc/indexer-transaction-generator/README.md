# Indexer Transaction Generator

This tool is to generate transactions for testing purposes.

## Usage

Under the root folder, i.e., `velor-core`, run the follow command. This will default to importing transactions for all networks.

```bash
cargo run -p velor-indexer-transaction-generator -- --testing-folder ecosystem/indexer-grpc/indexer-transaction-generator/imported_transactions --output-folder ecosystem/indexer-grpc/indexer-test-transactions/src
```

You can optionally specify the mode, e.g. for script mode

```bash
cargo run -p velor-indexer-transaction-generator -- --testing-folder ecosystem/indexer-grpc/indexer-transaction-generator/imported_transactions --output-folder ecosystem/indexer-grpc/indexer-test-transactions/src --mode script
```

Or network, e.g. mainnet

```bash
cargo run -p velor-indexer-transaction-generator -- --testing-folder ecosystem/indexer-grpc/indexer-transaction-generator/imported_transactions --output-folder ecosystem/indexer-grpc/indexer-test-transactions/src --network mainnet
```

Or testnet

```bash
cargo run -p velor-indexer-transaction-generator -- --testing-folder ecosystem/indexer-grpc/indexer-transaction-generator/imported_transactions --output-folder ecosystem/indexer-grpc/indexer-test-transactions/src --network testnet
```

### Config overview

Your testing folder should contain:
- One file called `testing_accounts.yaml`, which contains testing accounts used.
    ```yaml
    accounts:
      a531b7fdd7917f73ca216d89a8d9ce0cf7e7cfb9086ca6f6cbf9521532748d16:
        private_key: "0x99978d48e7b2d50d0a7a3273db0929447ae59635e71118fa256af654c0ce56c9"
        public_key: "0x39b4acc85e026dc056464a5ea00b98f858260eaad2b74dd30b86ae0d4d94ddf5"
        account: a531b7fdd7917f73ca216d89a8d9ce0cf7e7cfb9086ca6f6cbf9521532748d16
    ```
- One file called `imported_transactions.yaml`, which is used for importing transactions.
    
    ```yaml
    testnet:
      # Transaction Stream endpoint address.
      transaction_stream_endpoint: https://grpc.testnet.velorlabs.com:443
      # (Optional) The key to use with developers.velorlabs.com
      api_key: YOUR_KEY_HERE
      # A map from versions to dump and their output names.
      versions_to_import:
        123: testnet_v1.json
    mainnet:
      ...    
    ```
- One folder called `move_fixtures`, which contains move scripts and configs.
    * An example script transaction config looks like:
    ```yaml
    transactions:
      - output_name: fa_mint_transfer_burn
        script_path: fa_mint_transfer_burn
        sender_address: REPLACE_WITH_ACCOUNT_ADDRESS
    ``` 

You can check the example [here](imported_transactions).

### Account Management

Each `sender_address` specified in the script transaction config is a placeholder string; 
the actual account address will be allocated by the account manager.

The accounts in `testing_accounts.yaml` will be used to run scripted transactions. 
They are persisted in the config so each scripted transaction's generated output stays consistent between 
`velor-indexer-transaction-generator` runs. You can generate more testing accounts using 
Velor CLI by running `velor init --profile local`. 

TODO: account manager handles address as script argument.


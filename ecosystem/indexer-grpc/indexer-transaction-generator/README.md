# Indexer Transaction Generator

This tool is to generate transactions for testing purpose.

## Usage

Under root folder, i.e., `aptos-core`, run

```bash
cargo run -p aptos-indexer-transaction-generator -- \
  --testing-folder ecosystem/indexer-grpc/indexer-transaction-generator/example_tests \ 
  --output-folder ecosystem/indexer-grpc/indexer-transaction-generator/example_tests
```

**You can also use absolute path, run(using binary as an example)**

```bash
./aptos-indexer-transaction-generator \
  --testing-folder /your/aptos-core/ecosystem/indexer-grpc/indexer-transaction-generator/example_tests \ 
  --output-folder /tmp/ttt
```

### Config overview

* Your testing folder should contain:
  * One file called `testing_accounts.yaml`, which contains testing accounts used.
      ```yaml
      accounts:
        - private_key: "0x99978d48e7b2d50d0a7a3273db0929447ae59635e71118fa256af654c0ce56c9"
          public_key: "0x39b4acc85e026dc056464a5ea00b98f858260eaad2b74dd30b86ae0d4d94ddf5"
          account: a531b7fdd7917f73ca216d89a8d9ce0cf7e7cfb9086ca6f6cbf9521532748d16
        - ...
      ```
  * One file called `imported_transactions.yaml`, which is used for importing transactions.
    
      ```yaml
      testnet:
        # Transaction Stream endpoint addresss.
        transaction_stream_endpoint: https://grpc.testnet.aptoslabs.com:443
        # (Optional) The key to use with developers.aptoslabs.com
        api_key: YOUR_KEY_HERE
        # A map from versions to dump and their output names.
        versions_to_import:
          123: testnet_v1.json
      mainnet:
        ...    
      ```
  * One folder called `move_fixtures`, which contains move scripts and configs.
    * An example script transaction config looks like:
      ```yaml
      transactions:
        - output_name: simple_user_script1
          script_path: simple_user_script
          sender_address: __ACCOUNT_A__
        - output_name: simple_user_script2
          script_path: simple_user_script2
          sender_address: __ACCOUNT_A__
      ``` 


You can check the example [here](example_tests).


### Account Management
Each sender_address specified in script transaction config is a place holder string; 
the actual account address will be allocated by account manager.

TODO: account manager handles address as script argument.


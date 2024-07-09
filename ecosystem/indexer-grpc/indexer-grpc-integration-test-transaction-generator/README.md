# Indexer Integration Tests Transaction Generator

* Indexer Integration Tests Transaction Generator is to generate transactions
  based on input Move files.

## How to use it

```
cargo run -- --test-cases-folder TEST_CASES_PATH --output-test-cases-folder OUTPUT_PATH --aptos-node-binary APTOS_NODE_BINARY_PATH
```

## Design 

```
Test Cases                                       Txns          
                                                                     
┌─────────┐            ┌────────────────┐        ┌─────────┐         
│         │            │                │        │         │         
│   ┌─────┼───┐ ──────►│ Txn Generator  │ ─────► │   ┌─────┼───┐     
│   │     │   │        │ ┌────────────┐ │        │   │     │   │     
└───┼────┬┴───┼────┐   │ │ Fullnode   │ │        └───┼────┬┴───┼────┐
    │    │    │    │   │ │            │ │            │    │    │    │
    └────┼────┘    │   │ └────────────┘ │            └────┼────┘    │
         │         │   │                │                 │         │
         └─────────┘   └────────────────┘                 └─────────┘

```

## Config

* `--test-cases-folder`
  * Path to the main folder that stores all the test cases.
*  `--output-test-cases-folder`
  * Path to store the generated transactions based on test cases.
* `--aptos-node-binary`
  * Path to the Aptos Node binary.
* `--node-config`
  * Optional: the path to the local node config for generating transactions; otherwise, use default config.
* `--node-version`
  * Optional: the node version to generate transactions; default is `main`.

## Folder structure

* Test case folder  **must** start with `test_case_` otherwise, it'll be ignored.
* Under each test case, folders need to be prefixed with `setup_` or `step` followed by number. Otherwise, it'll be errors.
  * Setup is optional. 

```

test_cases_folder 
└─test_case_1
  └─setup_1
  └─step_1
└─test_case_2
└─test_case_your_own_name

  




```
---
title: "Tests and Validation"
slug: "typescript-sdk-tests"
---

The TypeScript SDK uses two types of tests, `e2e` and `unit` tests, located under the `src/tests/` folder:

- `e2e` tests – End-to-end tests are meant to test the end-to-end operations starting from the SDK methods to the interaction with the REST/Indexer API and a smart contract and up to the blockchain level. For example, to test if a transaction has been submitted, we start with building the transaction payload the SDK expects, post the submit request to the REST API, and fetch the transaction data to make sure it has been fully submitted to the blockchain.
- `unit` tests – Unit tests are meant to test the output of a function in the SDK with the provided input. For example, we can test whether an account address is valid.

## Validation for the Transaction Builder and BCS

The [BCS](https://docs.rs/bcs/latest/bcs/) is used to assemble and serialize the transaction payloads for signing and submission.

Given that different programming languages have different primitive type constraints (e.g., byte length, value range, etc.) and various composite types support (e.g., enum, struct, class, etc.), the code for data serialization is hard to validate.

The Aptos SDK validates the Transaction Builder and BCS in two ways:

1. The first level of validation is through unit tests and end-to-end (e2e) tests.

:::tip

An example of unit tests for the BCS serializer can be found in [`serializer.test.ts`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/tests/unit/serializer.test.ts).

An example of an e2e test for submitting a BCS transaction can be found in [`aptos_client.test.ts`](https://github.com/aptos-labs/aptos-core/blob/f4a7820a61f22ed8306219621402d96f70379d20/ecosystem/typescript/sdk/src/tests/e2e/aptos_client.test.ts#L78).

:::

2. The second level of validation is fuzzing tests with test vectors. The test vectors are produced by the same code used by the Aptos blockchain. The test vectors are arrays of JSON objects. Each JSON object contains randomized inputs and the expected outputs. The Aptos SDKs can parse and load test vectors to validate their implementations of Transaction Builder and BCS.

There are a two test vectors. Each covers one type of transaction payload:

- [EntryFunction](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_entry_function_payload.json) vector
- [Script](https://github.com/aptos-labs/aptos-core/blob/main/api/goldens/aptos_api__tests__transaction_vector_test__test_script_payload.json) vector

Vector items are self-explanatory. However, a special serialization method is used to save space and avoid data overflow. The details are described below:

- All account address are hex-coded.
- `args` in EntryFunction is hex-coded.
- U64 and U128 numbers are serialized as string literals to avoid data truncation.
- U8 is serialized as a number (not a string).
- `code` in Script and ModuleBundle are hex-coded.

:::tip
See the [`transaction_vector.test.ts`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/tests/unit/transaction_vector.test.ts) code example for how the TypeScript SDK does vector validation.
:::

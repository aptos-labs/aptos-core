---
title: "Typescript SDK Validation"
slug: "typescript-sdk-validation"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

## Validation for the Transaction Builder and BCS

The [BCS](https://docs.rs/bcs/latest/bcs/) is used to assemble and serialize the transaction payloads for signing and submission.

Given that different programming languages have different primitive type constraints (e.g., byte length, value range, etc.) and various composite types support (e.g., enum, struct, class, etc.), the code for data serialization is hard to validate.

The Aptos SDK validates the Transaction Builder and BCS in two ways:

1. First, with the unit tests and end-to-end (e2e) tests.

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

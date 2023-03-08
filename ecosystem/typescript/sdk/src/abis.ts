// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable max-len */

// ecosystem/typescript/sdk/src/move_scripts/token_transfer_with_opt_in
export const TOKEN_TRANSFER_OPT_IN =
  "a11ceb0b0500000006010004020408030c0a05161d073339086c400000010100020700010307000104030100010504020007060c0508000800030503010801000405080008000304060c0801050306737472696e6705746f6b656e06537472696e6707546f6b656e4964136372656174655f746f6b656e5f69645f726177087472616e73666572000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000030000010c0b010b020b030b0411000c070b000b070b050b06110102";

/*
Follow these steps to get the ABI strings:

Go to the package directory of the relevant Move module, e.g. if you're trying
to get the ABI for the `transfer` function of `aptos_account.move`, go to
the directory `aptos-move/framework/aptos-framework`.

Compile the Move packages with the Aptos CLI:
```
aptos move compile --included-artifacts all
```
This `--included-artifacts all` argument is necessary to generate ABIs.

Find the ABI files under the `build` directory and convert the ABI files to hex strings.
On Mac and Linux, this can be done with this command:
```
cat <ABI_FILE_PATH> | od -v -t x1 -A n | tr -d ' \n'
```
For example:
```
cat build/AptosFramework/abis/aptos_account/transfer.abi | od -v -t x1 -A n | tr -d ' \n'
```
*/

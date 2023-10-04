// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from "../../providers/aptos_client";
import { getFaucetClient, longTestTimeout, NODE_URL } from "../unit/test_helper.test";
import { AptosAccount } from "../../account/aptos_account";
import { COIN_TRANSFER, CoinClient, TRANSFER_COINS } from "../../plugins/coin_client";
import { EntryFunctionPayload, Transaction_UserTransaction } from "../../generated";
import { APTOS_COIN } from "../../utils";

test(
  "transfer and checkBalance works",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = getFaucetClient();
    const coinClient = new CoinClient(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();
    await faucetClient.fundAccount(alice.address(), 100_000_000);
    await faucetClient.fundAccount(bob.address(), 0);

    const txnHash1 = await coinClient.transfer(alice, bob, 42, { coinType: APTOS_COIN });
    await client.waitForTransaction(txnHash1, { checkSuccess: true });

    expect(await coinClient.checkBalance(bob, { coinType: APTOS_COIN })).toBe(BigInt(42));
    let txn1 = (await client.getTransactionByHash(txnHash1)) as Transaction_UserTransaction;
    expect((txn1.payload as EntryFunctionPayload).function).toBe(TRANSFER_COINS);

    // Test that `createReceiverIfMissing` works.
    const jemima = new AptosAccount();
    const txnHash2 = await coinClient.transfer(alice, jemima, 717, { createReceiverIfMissing: true });
    await client.waitForTransaction(txnHash2, {
      checkSuccess: true,
    });

    // Check that using a string address instead of an account works with `checkBalance`.
    expect(await coinClient.checkBalance(jemima.address().hex())).toBe(BigInt(717));
    let txn2 = (await client.getTransactionByHash(txnHash2)) as Transaction_UserTransaction;
    expect((txn2.payload as EntryFunctionPayload).function).toBe(TRANSFER_COINS);

    // Test that `createReceiverIfMissing` works off (has to already be registered
    const txnHash3 = await coinClient.transfer(alice, jemima, 1234, { createReceiverIfMissing: false });
    await client.waitForTransaction(txnHash3, {
      checkSuccess: true,
    });

    expect(await coinClient.checkBalance(jemima.address().hex())).toBe(BigInt(1951));
    let txn3 = (await client.getTransactionByHash(txnHash3)) as Transaction_UserTransaction;
    expect((txn3.payload as EntryFunctionPayload).function).toBe(COIN_TRANSFER);
  },
  longTestTimeout,
);

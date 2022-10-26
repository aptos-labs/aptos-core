// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from "./aptos_client";
import { FaucetClient } from "./faucet_client";
import { AptosAccount } from "./aptos_account";
import { HexString } from "./hex_string";
import * as Gen from "./generated/index";

import { NODE_URL, getFaucetClient, longTestTimeout } from "./utils/test_helper.test";

const aptosCoin = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";

test("faucet url empty", () => {
  expect(() => {
    const faucetClient = new FaucetClient("http://localhost:8080", "");
    faucetClient.getAccount("0x1");
  }).toThrow("Faucet URL cannot be empty.");
});

test(
  "full tutorial faucet flow",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = getFaucetClient();

    const account1 = new AptosAccount();
    const txns = await faucetClient.fundAccount(account1.address(), 10000000);
    const tx0 = await client.getTransactionByHash(txns[0]);
    expect(tx0.type).toBe("user_transaction");
    let resources = await client.getAccountResources(account1.address());
    let accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as { coin: { value: string } }).coin.value).toBe("10000000");

    const account2 = new AptosAccount();
    await faucetClient.fundAccount(account2.address(), 0);
    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as { coin: { value: string } }).coin.value).toBe("0");

    const payload: Gen.TransactionPayload_EntryFunctionPayload = {
      type: "entry_function_payload",
      function: "0x1::coin::transfer",
      type_arguments: ["0x1::aptos_coin::AptosCoin"],
      arguments: [account2.address().hex(), 717],
    };

    const txnRequest = await client.generateTransaction(account1.address(), payload, { max_gas_amount: "2000" });
    const signedTxn = await client.signTransaction(account1, txnRequest);
    const transactionRes = await client.submitTransaction(signedTxn);
    const txn = await client.waitForTransactionWithResult(transactionRes.hash);
    expect((txn as any)?.success).toBe(true);

    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as { coin: { value: string } }).coin.value).toBe("717");

    const res = await client.getAccountTransactions(account1.address(), { start: BigInt(0) });
    const tx = res.find((e) => e.type === "user_transaction") as Gen.UserTransaction;
    expect(new HexString(tx.sender).toShortString()).toBe(account1.address().toShortString());

    const events = await client.getEventsByEventHandle(tx.sender, aptosCoin, "withdraw_events");
    expect(events[0].type).toBe("0x1::coin::WithdrawEvent");

    const eventSubset = await client.getEventsByEventHandle(tx.sender, aptosCoin, "withdraw_events", {
      start: BigInt(0),
      limit: 1,
    });
    expect(eventSubset[0].type).toBe("0x1::coin::WithdrawEvent");

    const events2 = await client.getEventsByCreationNumber(
      events[0].guid.account_address,
      events[0].guid.creation_number,
    );
    expect(events2[0].type).toBe("0x1::coin::WithdrawEvent");
  },
  longTestTimeout,
);

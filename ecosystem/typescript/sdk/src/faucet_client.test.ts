import { AptosClient } from "./aptos_client";
import { FaucetClient } from "./faucet_client";
import { AptosAccount } from "./aptos_account";
import { Types } from "./types";
import { UserTransaction } from "./api/data-contracts";
import { HexString } from "./hex_string";

import { NODE_URL, FAUCET_URL } from "./util.test";

test(
  "full tutorial faucet flow",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

    const account1 = new AptosAccount();
    const txns = await faucetClient.fundAccount(account1.address(), 5000);
    const tx1 = await client.getTransaction(txns[1]);
    expect(tx1.type).toBe("user_transaction");
    let resources = await client.getAccountResources(account1.address());
    let accountResource = resources.find((r) => r.type === "0x1::TestCoin::Balance");
    expect((accountResource.data as { coin: { value: string } }).coin.value).toBe("5000");

    const account2 = new AptosAccount();
    await faucetClient.fundAccount(account2.address(), 0);
    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === "0x1::TestCoin::Balance");
    expect((accountResource.data as { coin: { value: string } }).coin.value).toBe("0");

    const payload: Types.TransactionPayload = {
      type: "script_function_payload",
      function: "0x1::TestCoin::transfer",
      type_arguments: [],
      arguments: [account2.address().hex(), "717"],
    };
    const txnRequest = await client.generateTransaction(account1.address(), payload);
    const signedTxn = await client.signTransaction(account1, txnRequest);
    const transactionRes = await client.submitTransaction(signedTxn);
    await client.waitForTransaction(transactionRes.hash);

    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === "0x1::TestCoin::Balance");
    expect((accountResource.data as { coin: { value: string } }).coin.value).toBe("717");

    const res = await client.getAccountTransactions(account1.address(), { start: 0 });
    const tx = res.find((e) => e.type === "user_transaction") as UserTransaction;
    expect(new HexString(tx.sender).toShortString()).toBe(account1.address().toShortString());

    const events = await client.getEventsByEventHandle(tx.sender, "0x1::TestCoin::TransferEvents", "sent_events");
    expect(events[0].type).toBe("0x1::TestCoin::SentEvent");

    const event_subset = await client.getEventsByEventHandle(
      tx.sender,
      "0x1::TestCoin::TransferEvents",
      "sent_events",
      { start: 0, limit: 1 },
    );
    expect(event_subset[0].type).toBe("0x1::TestCoin::SentEvent");

    const events2 = await client.getEventsByEventKey(events[0].key);
    expect(events2[0].type).toBe("0x1::TestCoin::SentEvent");
  },
  30 * 1000,
);

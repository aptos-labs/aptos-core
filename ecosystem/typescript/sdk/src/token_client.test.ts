// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { FaucetClient } from "./faucet_client";
import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import { TokenClient } from "./token_client";

import { NODE_URL, FAUCET_URL } from "./util.test";

test(
  "full tutorial nft token flow",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);
    const tokenClient = new TokenClient(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();

    // Fund both Alice's and Bob's Account
    await faucetClient.fundAccount(alice.address(), 10000000);
    await faucetClient.fundAccount(bob.address(), 10000000);

    const collectionName = "AliceCollection";
    const tokenName = "Alice Token";

    async function ensureTxnSuccess(txnHashPromise: Promise<string>) {
      const txnHash = await txnHashPromise;
      const txn = await client.waitForTransactionWithResult(txnHash);
      expect((txn as any)?.success).toBe(true);
    }

    // Create collection and token on Alice's account
    await ensureTxnSuccess(
      tokenClient.createCollection(alice, collectionName, "Alice's simple collection", "https://aptos.dev"),
    );

    await ensureTxnSuccess(
      tokenClient.createToken(
        alice,
        collectionName,
        tokenName,
        "Alice's simple token",
        1,
        "https://aptos.dev/img/nyan.jpeg",
        1000,
        alice.address(),
        0,
        0,
        ["key"],
        ["2"],
        ["int"],
      ),
    );

    const tokenId = {
      token_data_id: {
        creator: alice.address().hex(),
        collection: collectionName,
        name: tokenName,
      },
      property_version: "0",
    };

    // Transfer Token from Alice's Account to Bob's Account
    await tokenClient.getCollectionData(alice.address().hex(), collectionName);
    let aliceBalance = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("1");
    const tokenData = await tokenClient.getTokenData(alice.address().hex(), collectionName, tokenName);
    expect(tokenData.name).toBe(tokenName);

    await ensureTxnSuccess(
      tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName, 1),
    );
    aliceBalance = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("0");

    await ensureTxnSuccess(
      tokenClient.cancelTokenOffer(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName),
    );
    aliceBalance = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("1");

    await ensureTxnSuccess(
      tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName, 1),
    );
    aliceBalance = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("0");

    await ensureTxnSuccess(
      tokenClient.claimToken(bob, alice.address().hex(), alice.address().hex(), collectionName, tokenName),
    );

    const bobBalance = await tokenClient.getTokenForAccount(bob.address().hex(), tokenId);
    expect(bobBalance.amount).toBe("1");
  },
  30 * 1000,
);

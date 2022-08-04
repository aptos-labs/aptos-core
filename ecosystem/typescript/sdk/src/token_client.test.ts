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
    await faucetClient.fundAccount(alice.address(), 1000000);
    await faucetClient.fundAccount(bob.address(), 1000000);

    const collectionName = "AliceCollection";
    const tokenName = "Alice Token";

    // Create collection and token on Alice's account
    let txnHash1 = await tokenClient.createCollection(
      alice,
      collectionName,
      "Alice's simple collection",
      "https://aptos.dev",
    );
    const txn1 = await client.waitForTransactionWithResult(txnHash1);
    expect((txn1 as any)?.success).toBe(true);

    let txnHash2 = await tokenClient.createToken(
      alice,
      collectionName,
      tokenName,
      "Alice's simple token",
      1,
      "https://aptos.dev/img/nyan.jpeg",
      alice.address(),
      0,
      0,
      ["key"],
      ["2"],
      ["int"],
    );
    const txn2 = await client.waitForTransactionWithResult(txnHash2);
    expect((txn2 as any)?.success).toBe(true);

    const tokenId = {
      token_data_id: {
        creator: alice.address().hex(),
        collection: Buffer.from(collectionName).toString("hex"),
        name: Buffer.from(tokenName).toString("hex"),
      },
      property_version: "0",
    };

    // Transfer Token from Alice's Account to Bob's Account
    await tokenClient.getCollectionData(alice.address().hex(), collectionName);
    let aliceBalance = await tokenClient.getTokenBalanceForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("1");
    const tokenData = await tokenClient.getTokenData(alice.address().hex(), collectionName, tokenName);
    expect(tokenData.name).toBe(Buffer.from(tokenName).toString("hex"));

    await tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName, 1);
    aliceBalance = await tokenClient.getTokenBalanceForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("0");

    await tokenClient.cancelTokenOffer(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName);
    aliceBalance = await tokenClient.getTokenBalanceForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("1");

    await tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName, 1);
    aliceBalance = await tokenClient.getTokenBalanceForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("0");

    await tokenClient.claimToken(bob, alice.address().hex(), alice.address().hex(), collectionName, tokenName);

    const bobBalance = await tokenClient.getTokenBalanceForAccount(bob.address().hex(), tokenId);
    expect(bobBalance.amount).toBe("1");
  },
  30 * 1000,
);

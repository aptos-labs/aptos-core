// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import { TokenClient } from "./token_client";

import { getFaucetClient, longTestTimeout, NODE_URL } from "./utils/test_helper.test";
import { bcsSerializeBool } from "./bcs";
import { deserializePropertyMap } from "./utils/property_map_serde";

test(
  "full tutorial nft token flow",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = getFaucetClient();
    const tokenClient = new TokenClient(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();

    // Fund both Alice's and Bob's Account
    await faucetClient.fundAccount(alice.address(), 100000000);
    await faucetClient.fundAccount(bob.address(), 100000000);

    const collectionName = "AliceCollection";
    const tokenName = "Alice Token";

    // Create collection and token on Alice's account
    await client.waitForTransaction(
      await tokenClient.createCollection(alice, collectionName, "Alice's simple collection", "https://aptos.dev"),
      { checkSuccess: true },
    );

    await client.waitForTransaction(
      await tokenClient.createTokenWithMutabilityConfig(
        alice,
        collectionName,
        tokenName,
        "Alice's simple token",
        2,
        "https://aptos.dev/img/nyan.jpeg",
        1000,
        alice.address(),
        1,
        0,
        ["TOKEN_BURNABLE_BY_OWNER"],
        [bcsSerializeBool(true)],
        ["bool"],
        [false, false, false, false, true],
      ),
      { checkSuccess: true },
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
    expect(aliceBalance.amount).toBe("2");
    const tokenData = await tokenClient.getTokenData(alice.address().hex(), collectionName, tokenName);
    expect(tokenData.name).toBe(tokenName);

    await client.waitForTransaction(
      await tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName, 1),
      { checkSuccess: true },
    );
    aliceBalance = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("1");

    await client.waitForTransaction(
      await tokenClient.cancelTokenOffer(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName),
      { checkSuccess: true },
    );
    aliceBalance = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("2");

    await client.waitForTransaction(
      await tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collectionName, tokenName, 1),
      { checkSuccess: true },
    );
    aliceBalance = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
    expect(aliceBalance.amount).toBe("1");

    await client.waitForTransaction(
      await tokenClient.claimToken(bob, alice.address().hex(), alice.address().hex(), collectionName, tokenName),
      { checkSuccess: true },
    );

    const bobBalance = await tokenClient.getTokenForAccount(bob.address().hex(), tokenId);
    expect(bobBalance.amount).toBe("1");

    // default token property is configured to be mutable and then alice can make bob burn token after token creation
    // test mutate Bob's token properties and allow owner to burn this token
    let a = await tokenClient.mutateTokenProperties(
      alice,
      bob.address(),
      alice.address(),
      collectionName,
      tokenName,
      0,
      1,
      ["test"],
      [bcsSerializeBool(true)],
      ["bool"],
    );
    await client.waitForTransactionWithResult(a);

    const newTokenId = {
      token_data_id: {
        creator: alice.address().hex(),
        collection: collectionName,
        name: tokenName,
      },
      property_version: "1",
    };
    const mutated_token = await tokenClient.getTokenForAccount(bob.address().hex(), newTokenId);
    // expect property map deserialization works
    expect(mutated_token.token_properties.data["test"].value).toBe("true");
    expect(mutated_token.token_properties.data["TOKEN_BURNABLE_BY_OWNER"].value).toBe("true");

    // burn the token by owner
    var txn_hash = await tokenClient.burnByOwner(bob, alice.address(), collectionName, tokenName, 1, 1);
    await client.waitForTransactionWithResult(txn_hash);
    const newbalance = await tokenClient.getTokenForAccount(bob.address().hex(), newTokenId);
    expect(newbalance.amount).toBe("0");

    //bob opt_in directly transfer and alice transfer token to bob directly
    txn_hash = await tokenClient.optInTokenTransfer(bob, true);
    await client.waitForTransactionWithResult(txn_hash);

    // alice still have one token with property version 0.
    txn_hash = await tokenClient.transferWithOptIn(
      alice,
      alice.address(),
      collectionName,
      tokenName,
      0,
      bob.address(),
      1,
    );
    await client.waitForTransactionWithResult(txn_hash);
    const balance = await tokenClient.getTokenForAccount(bob.address().hex(), tokenId);
    expect(balance.amount).toBe("1");
  },
  longTestTimeout,
);

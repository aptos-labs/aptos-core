// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosClient, AptosAccount, FaucetClient, TokenClient, CoinClient } from "aptos";
import { NODE_URL, FAUCET_URL } from "./common";

(async () => {
  // Helper for waiting on a transaction and ensuring it was successful.
  async function ensureTxnSuccess(txnHash: string) {
    const txn = await client.waitForTransactionWithResult(txnHash);
    if (!(txn as any)?.success) {
      throw `Transaction ${txnHash} failed: ${JSON.stringify(txn)}`;
    }
  }

  // Create API and faucet clients.
  // :!:>section_1a
  const client = new AptosClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL); // <:!:section_1a

  // Create client for working with the token module.
  // :!:>section_1b
  const tokenClient = new TokenClient(client); // <:!:section_1b

  // Create a coin client for checking account balances.
  const coinClient = new CoinClient(client);

  // Create accounts.
  // :!:>section_2
  const alice = new AptosAccount();
  const bob = new AptosAccount(); // <:!:section_2

  // Print out account addresses.
  console.log("=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);
  console.log(`Alice: ${bob.address()}`);
  console.log("");

  // Fund accounts.
  // :!:>section_3
  await faucetClient.fundAccount(alice.address(), 20_000);
  await faucetClient.fundAccount(bob.address(), 20_000); // <:!:section_3

  console.log("=== Initial Balances ===");
  console.log(`Alice: ${await coinClient.checkBalance(alice)}`);
  console.log(`Bob: ${await coinClient.checkBalance(bob)}`);
  console.log("");

  console.log("=== Creating Collection and Token ===");

  const collectionName = "Alice's";
  const tokenName = "Alice's first token";
  const tokenPropertyVersion = 0;

  const tokenId = {
    token_data_id: {
      creator: alice.address().hex(),
      collection: collectionName,
      name: tokenName,
    },
    property_version: `${tokenPropertyVersion}`,
  };

  // Create the collection.
  // :!:>section_4
  const txnHash1 = await tokenClient.createCollection(
    alice,
    collectionName,
    "Alice's simple collection",
    "https://alice.com",
  ); // <:!:section_4
  await ensureTxnSuccess(txnHash1);

  // Create a token in that collection.
  // :!:>section_5
  const txnHash2 = await tokenClient.createToken(
    alice,
    collectionName,
    tokenName,
    "Alice's simple token",
    1,
    "https://aptos.dev/img/nyan.jpeg",
  ); // <:!:section_5
  await ensureTxnSuccess(txnHash2);

  // Print the collection data.
  // :!:>section_6
  const collectionData = await tokenClient.getCollectionData(alice.address(), collectionName);
  console.log(`Alice's collection: ${JSON.stringify(collectionData, null, 4)}`); // <:!:section_6

  // Get the token balance.
  // :!:>section_7
  const balance1 = await tokenClient.getTokenBalance(
    alice.address(),
    collectionName,
    tokenName,
    `${tokenPropertyVersion}`,
  );
  console.log(`Alice's token balance: ${balance1["amount"]}`); // <:!:section_7

  // Get the token data.
  // :!:>section_8
  const tokenData = await tokenClient.getTokenData(alice.address(), collectionName, tokenName);
  console.log(`Alice's token data: ${JSON.stringify(tokenData, null, 4)}`); // <:!:section_8

  // Alice offers one token to Bob.
  // :!:>section_9
  console.log("\n=== Transferring the token to Bob ===");
  const txnHash3 = await tokenClient.offerToken(
    alice,
    bob.address(),
    alice.address(),
    collectionName,
    tokenName,
    1,
    tokenPropertyVersion,
  ); // <:!:section_9
  await ensureTxnSuccess(txnHash3);

  // Bob claims the token Alice offered him.
  // :!:>section_10
  const txnHash4 = await tokenClient.claimToken(
    bob,
    alice.address(),
    alice.address(),
    collectionName,
    tokenName,
    tokenPropertyVersion,
  ); // <:!:section_10
  await ensureTxnSuccess(txnHash4);

  // todo
  // console.log("\n=== Transferring the token back to Alice using MultiAgent ===");
  // :!:>section_11
  // Coming soon, we don't have direct_transfer_script wrapped in the TS token client quite yet.
  // <:!:section_11

  // Print out their balances one last time.
  const aliceBalanceFinal = await tokenClient.getTokenBalance(
    alice.address(),
    collectionName,
    tokenName,
    `${tokenPropertyVersion}`,
  );
  const bobBalanceFinal = await tokenClient.getTokenBalanceForAccount(
    bob.address(),
    tokenId,
  );
  console.log(`Alice's token balance: ${aliceBalanceFinal["amount"]}`);
  console.log(`Bob's token balance: ${bobBalanceFinal["amount"]}`);
})();

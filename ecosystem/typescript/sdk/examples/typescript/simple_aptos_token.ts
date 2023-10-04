// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosAccount, FaucetClient, AptosToken, CoinClient, Network, Provider, HexString } from "aptos";
import { NODE_URL, FAUCET_URL } from "./common";

(async () => {
  // Create API and faucet clients.
  // :!:>section_1a
  const provider = new Provider(Network.DEVNET);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL); // <:!:section_1a

  // Create client for working with the token module.
  // :!:>section_1b
  const aptosTokenClient = new AptosToken(provider); // <:!:section_1b

  // Create a coin client for checking account balances.
  const coinClient = new CoinClient(provider.aptosClient);

  // Create accounts.
  // :!:>section_2
  const alice = new AptosAccount();
  const bob = new AptosAccount(); // <:!:section_2

  // Print out account addresses.
  console.log("=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);
  console.log(`Bob: ${bob.address()}`);
  console.log("");

  // Fund accounts.
  // :!:>section_3
  await faucetClient.fundAccount(alice.address(), 100_000_000);
  await faucetClient.fundAccount(bob.address(), 100_000_000); // <:!:section_3

  console.log("=== Initial Coin Balances ===");
  console.log(`Alice: ${await coinClient.checkBalance(alice)}`);
  console.log(`Bob: ${await coinClient.checkBalance(bob)}`);
  console.log("");

  console.log("=== Creating Collection and Token ===");

  const collectionName = "Alice's";
  const tokenName = "Alice's first token";
  const maxSupply = 1;

  // Create the collection.
  // :!:>section_4
  const txnHash1 = await aptosTokenClient.createCollection(
    alice,
    "Alice's simple collection",
    collectionName,
    "https://alice.com",
    maxSupply,
    {
      royaltyNumerator: 5,
      royaltyDenominator: 100,
    },
  ); // <:!:section_4
  await provider.aptosClient.waitForTransaction(txnHash1, { checkSuccess: true });

  // Create a token in that collection.
  // :!:>section_5
  const txnHash2 = await aptosTokenClient.mint(
    alice,
    collectionName,
    "Alice's simple token",
    tokenName,
    "https://aptos.dev/img/nyan.jpeg",
    [],
    [],
    [],
  ); // <:!:section_5
  await provider.aptosClient.waitForTransaction(txnHash2, { checkSuccess: true });

  const inSync = await ensureIndexerAndNetworkInSync(provider);
  if (!inSync) {
    return;
  }

  // Print the collection data.
  // :!:>section_6
  const collectionData = (await provider.getCollectionData(alice.address(), collectionName)).current_collections_v2[0];
  console.log(`Alice's collection: ${JSON.stringify(collectionData, null, 4)}`); // <:!:section_6

  // Get the token balance.
  // :!:>section_7
  const collectionAddress = HexString.ensure(collectionData.collection_id);
  let { tokenAddress, amount: aliceAmount } = await getTokenInfo(provider, alice.address(), collectionAddress);
  console.log(`Alice's token balance: ${aliceAmount}`); // <:!:section_7

  // Get the token data.
  // :!:>section_8
  const tokenData = (await provider.getTokenData(tokenAddress.toString())).current_token_datas_v2[0];
  console.log(`Alice's token data: ${JSON.stringify(tokenData, null, 4)}`); // <:!:section_8

  // Alice transfers the token to Bob.
  console.log("\n=== Transferring the token to Bob ===");
  // :!:>section_9
  const txnHash3 = await aptosTokenClient.transferTokenOwnership(alice, tokenAddress, bob.address()); // <:!:section_9
  await provider.aptosClient.waitForTransaction(txnHash3, { checkSuccess: true });

  // Print their balances.
  // :!:>section_10
  aliceAmount = (await getTokenInfo(provider, alice.address(), collectionAddress)).amount;
  let bobAmount = (await getTokenInfo(provider, bob.address(), collectionAddress)).amount;
  console.log(`Alice's token balance: ${aliceAmount}`);
  console.log(`Bob's token balance: ${bobAmount}`); // <:!:section_10

  console.log("\n=== Transferring the token back to Alice ===");
  // :!:>section_11
  let txnHash4 = await aptosTokenClient.transferTokenOwnership(bob, tokenAddress, alice.address()); // <:!:section_11
  await provider.aptosClient.waitForTransaction(txnHash4, { checkSuccess: true });

  // :!:>section_12
  aliceAmount = (await getTokenInfo(provider, alice.address(), collectionAddress)).amount;
  bobAmount = (await getTokenInfo(provider, bob.address(), collectionAddress)).amount;
  console.log(`Alice's token balance: ${aliceAmount}`);
  console.log(`Bob's token balance: ${bobAmount}`); // <:!:section_12

  console.log("\n=== Getting Alices's NFTs ===");
  console.log(
    `Alice current token ownership: ${
      (await getTokenInfo(provider, alice.address(), collectionAddress)).amount
    }. Should be 1`,
  );

  console.log("\n=== Getting Bob's NFTs ===");
  console.log(
    `Bob current token ownership: ${
      (await getTokenInfo(provider, bob.address(), collectionAddress)).amount
    }. Should be 0\n`,
  );
})();

// :!:>getTokenInfo
async function getTokenInfo(
  provider: Provider,
  ownerAddress: HexString,
  collectionAddress: HexString,
): Promise<{ tokenAddress?: HexString; amount: number }> {
  const tokensOwnedQuery = await provider.getTokenOwnedFromCollectionAddress(
    ownerAddress,
    collectionAddress.toString(),
    {
      tokenStandard: "v2",
    },
  );
  const tokensOwned = tokensOwnedQuery.current_token_ownerships_v2.length;
  if (tokensOwned > 0) {
    return {
      tokenAddress: HexString.ensure(tokensOwnedQuery.current_token_ownerships_v2[0].current_token_data.token_data_id),
      amount: tokensOwnedQuery.current_token_ownerships_v2[0].amount,
    };
  } else {
    return {
      tokenAddress: undefined,
      amount: tokensOwned,
    };
  }
} // <:!:getTokenInfo

async function ensureIndexerAndNetworkInSync(provider: Provider): Promise<boolean> {
  const indexerLedgerInfo = await provider.getIndexerLedgerInfo();
  const fullNodeChainId = await provider.getChainId();
  if (indexerLedgerInfo.ledger_infos[0].chain_id !== fullNodeChainId) {
    console.log(`\nERROR: Provider's fullnode chain id and indexer chain id are not synced, skipping rest of tests`);
    return false;
  } else {
    return true;
  }
}

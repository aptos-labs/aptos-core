// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import {
  AptosAccount,
  FaucetClient,
  BCS,
  AptosClient,
  TokenClient,
  MaybeHexString,
  OptionalTransactionArgs,
  getPropertyValueRaw,
  Types,
} from "aptos";
import { NODE_URL, FAUCET_URL } from "./common";
import { assert } from "console";

const client = new AptosClient(NODE_URL);
const tokenClient = new TokenClient(client);
const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

async function waitForEnter() {
  return new Promise<void>((resolve, reject) => {
    const rl = require("readline").createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    rl.question("Please press the Enter key to proceed ...\n", () => {
      rl.close();
      resolve();
    });
  });
}

async function ensureTxnSuccess(txnHashPromise: Promise<string>) {
  const txnHash = await txnHashPromise;
  const txn = await client.waitForTransactionWithResult(txnHash);
  assert((txn as any)?.success);
}

const getBalance = async (account: AptosAccount) => {
  const resources = await client.getAccountResources(account.address().hex());
  const aptosCoin = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
  let accountResource = resources.find((r) => r.type === aptosCoin);
  return BigInt((accountResource!.data as any).coin.value);
};

// This function `createTokenWithFeePayer` allows `account` create a token while `feePayer` pays for the gas fees.
// To create a transaction with a different account paying for gas, we need to:
// 1. Create a fee payer raw transaction (TxnBuilderTypes.FeePayerRawTransaction) and
//    specify which account will be paying for gas.
// 2. When signing the transaction, two signatures are needed - one from the account sending the transaction and
//    one from the separate gas payer. In this example, the sender will sign first, followed by the gas payer.
//    However, the reverse order should also work. The signatures can be generated the same way they are for
//    normal transactions.
// 3. Once we have two signatures, the transaction can be sent, and gas fees will be deducted from
//    the gas payer account instead.
async function createTokenWithFeePayer(
  feePayer: AptosAccount,
  account: AptosAccount,
  collectionName: string,
  name: string,
  description: string,
  supply: number,
  uri: string,
  max: BCS.AnyNumber,
  royalty_payee_address?: MaybeHexString,
  royalty_points_denominator?: number,
  royalty_points_numerator?: number,
  property_keys?: Array<string>,
  property_values?: Array<string>,
  property_types?: Array<string>,
  extraArgs?: OptionalTransactionArgs,
): Promise<string> {
  const payload: Types.EntryFunctionPayload = {
    function: "0x3::token::create_token_script",
    type_arguments: [],
    arguments: [
      collectionName,
      name,
      description,
      supply,
      max,
      uri,
      royalty_payee_address,
      royalty_points_denominator,
      royalty_points_numerator,
      [false, false, false, false, false],
      property_keys,
      getPropertyValueRaw(property_values, property_types),
      property_types,
    ],
  };

  // Create a fee payer transaction with the sender, transaction payload, and fee payer account
  const feePayerTxn = await client.generateFeePayerTransaction(account.address().hex(), payload, feePayer.address());

  // sender and fee payer need to sign the transaction
  const senderAuthenticator = await client.signMultiTransaction(account, feePayerTxn);
  const feePayerAuthenticator = await client.signMultiTransaction(feePayer, feePayerTxn);

  // submit gas fee payer transaction
  const txn = await client.submitFeePayerTransaction(feePayerTxn, senderAuthenticator, feePayerAuthenticator);

  return txn.hash;
}

/** run our demo! */
async function main(): Promise<void> {
  // Generate Alice and Bob accounts.
  const alice = new AptosAccount();
  const bob = new AptosAccount();
  console.log("\n=== Alice and Bob accounts are randomly generated ===");
  console.log("Alice's profile:");
  console.log(`  alice:`);
  console.log(`    private_key: "${alice.toPrivateKeyObject().privateKeyHex}"`);
  console.log(`    public_key: "${alice.pubKey()}"`);
  console.log(`    account: ${alice.address()}`);
  console.log(`    rest_url: "https://fullnode.devnet.aptoslabs.com"`);
  console.log(`    faucet_url: "https://faucet.devnet.aptoslabs.com"`);

  console.log("Bob's profile:");
  console.log(`  bob:`);
  console.log(`    private_key: "${bob.toPrivateKeyObject().privateKeyHex}"`);
  console.log(`    public_key: "${bob.pubKey()}"`);
  console.log(`    account: ${bob.address()}`);
  console.log(`    rest_url: "https://fullnode.devnet.aptoslabs.com"`);
  console.log(`    faucet_url: "https://faucet.devnet.aptoslabs.com"`);
  await waitForEnter();

  // Fund Alice and Bob accounts.
  console.log("\n=== Alice and Bob accounts are funded ===");
  await faucetClient.fundAccount(alice.address(), 100_000_000);
  await faucetClient.fundAccount(bob.address(), 100_000_000);
  console.log(`Alice's balance: ${await getBalance(alice)} octas`);
  console.log(`Bob's balance: ${await getBalance(bob)} octas`);
  await waitForEnter();

  // Create a collection on Alice's account
  console.log("\n=== Alice sent a transaction to create a collection ===");
  const collectionName = "AliceCollection";
  let txnHash = await tokenClient.createCollection(
    alice,
    collectionName,
    "Alice's simple collection",
    "https://aptos.dev",
  );
  let response = await client.waitForTransactionWithResult(txnHash, { checkSuccess: true });
  let { gas_used, gas_unit_price } = response as any;
  console.log(`Alice paid the gas fee of ${gas_used * gas_unit_price} octas.`);
  console.log(`Alice's current balance: ${await getBalance(alice)} octas`);
  console.log(`Bob's current balance: ${await getBalance(bob)} octas`);
  await waitForEnter();

  // Create a token on Alice's account while Bob pays the fee.
  console.log("\n=== Alice sent a transaction to create a token while Bob paid the gas fee ===");
  const tokenName = "Alice Token";
  txnHash = await createTokenWithFeePayer(
    bob,
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
    ["u64"],
  );
  response = await client.waitForTransactionWithResult(txnHash, { checkSuccess: true });
  ({ gas_used, gas_unit_price } = response as any);
  const propertyVersion = 0;
  const tokenId = {
    token_data_id: {
      creator: alice.address().hex(),
      collection: collectionName,
      name: tokenName,
    },
    property_version: `${propertyVersion}`,
  };
  await tokenClient.getCollectionData(alice.address().hex(), collectionName);
  let aliceToken = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
  console.log(`Alice's token amount: ${aliceToken.amount}`);
  console.log(`Bob paid the gas fee of ${gas_used * gas_unit_price} octas.`);
  console.log(`Alice's current balance: ${await getBalance(alice)} octas`);
  console.log(`Bob's current balance: ${await getBalance(bob)} octas`);
  await waitForEnter();

  // Transfer Token from Alice's Account to Bob's Account with bob paying the fee
  console.log("\n=== Alice sent a transaction to send the token to Bob while Bob paid the gas fee ===");
  txnHash = await tokenClient.directTransferTokenWithFeePayer(
    alice,
    bob,
    alice.address(),
    collectionName,
    tokenName,
    1,
    bob,
    propertyVersion,
    undefined,
  );
  response = await client.waitForTransactionWithResult(txnHash, { checkSuccess: true });
  ({ gas_used, gas_unit_price } = response as any);
  aliceToken = await tokenClient.getTokenForAccount(alice.address().hex(), tokenId);
  const bobToken = await tokenClient.getTokenForAccount(bob.address().hex(), tokenId);
  console.log(`Alice's token amount: ${aliceToken.amount}`);
  console.log(`Bob's token amount: ${bobToken.amount}`);
  // Check that Alice did not pay the fee, but Bob did.
  console.log(`Bob paid the gas fee of ${gas_used * gas_unit_price} octas.`);
  console.log(`Alice's current balance: ${await getBalance(alice)} octas`);
  console.log(`Bob's current balance: ${await getBalance(bob)} octas`);
  await waitForEnter();
}

main().then(() => {
  console.log("Done!");
  process.exit(0);
});

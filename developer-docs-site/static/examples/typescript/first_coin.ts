// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import assert from "assert";
import fs from "fs";
import { NODE_URL, FAUCET_URL } from "./first_transaction";
import { AptosAccount, AptosClient, TxnBuilderTypes, BCS, MaybeHexString, HexString, FaucetClient } from "aptos";
import { publishModule } from "./hello_blockchain";

const readline = require("readline").createInterface({
  input: process.stdin,
  output: process.stdout,
});

//:!:>section_1
const client = new AptosClient(NODE_URL);
/** Initializes the new coin */
async function initializeCoin(accountFrom: AptosAccount, coinTypeAddress: HexString): Promise<string> {
  const token = new TxnBuilderTypes.TypeTagStruct(
    TxnBuilderTypes.StructTag.fromString(`${coinTypeAddress.hex()}::moon_coin::MoonCoin`),
  );

  const serializer = new BCS.Serializer();
  serializer.serializeBool(false);

  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x1::managed_coin",
      "initialize",
      [token],
      [BCS.bcsSerializeStr("Moon Coin"), BCS.bcsSerializeStr("MOON"), BCS.bcsSerializeUint64(6), serializer.getBytes()],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(accountFrom.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(accountFrom.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(accountFrom, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);

  return pendingTxn.hash;
}
//<:!:section_1

//:!:>section_2
/** Receiver needs to register the coin before they can receive it */
async function registerCoin(coinReceiver: AptosAccount, coinTypeAddress: HexString): Promise<string> {
  const token = new TxnBuilderTypes.TypeTagStruct(
    TxnBuilderTypes.StructTag.fromString(`${coinTypeAddress.hex()}::moon_coin::MoonCoin`),
  );

  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural("0x1::coins", "register", [token], []),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(coinReceiver.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(coinReceiver.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(coinReceiver, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);

  return pendingTxn.hash;
}
//<:!:section_2

//:!:>section_3
/** Mints the newly created coin to a specified receiver address */
async function mintCoin(
  coinOwner: AptosAccount,
  coinTypeAddress: HexString,
  receiverAddress: HexString,
  amount: number,
): Promise<string> {
  const token = new TxnBuilderTypes.TypeTagStruct(
    TxnBuilderTypes.StructTag.fromString(`${coinTypeAddress.hex()}::moon_coin::MoonCoin`),
  );

  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x1::managed_coin",
      "mint",
      [token],
      [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(receiverAddress.hex())), BCS.bcsSerializeUint64(amount)],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(coinOwner.address()),
    client.getChainId(),
  ]);

  const rawTxn = new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(coinOwner.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    1000n,
    1n,
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  const bcsTxn = AptosClient.generateBCSTransaction(coinOwner, rawTxn);
  const pendingTxn = await client.submitSignedBCSTransaction(bcsTxn);
  return pendingTxn.hash;
}
//<:!:section_3

//:!:>section_4
/** Return the balance of the newly created coin */
async function getBalance(accountAddress: MaybeHexString, coinTypeAddress: HexString): Promise<string | number> {
  try {
    const resource = await client.getAccountResource(
      accountAddress,
      `0x1::coin::CoinStore<${coinTypeAddress.hex()}::moon_coin::MoonCoin>`,
    );

    return parseInt((resource.data as any)["coin"]["value"]);
  } catch (_) {
    return 0;
  }
}
//<:!:section_4

/** run our demo! */
async function main() {
  assert(process.argv.length == 3, "Expecting an argument that points to the moon coin module");

  const client = new AptosClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

  // Create two accounts, Alice and Bob, and fund Alice but not Bob
  const alice = new AptosAccount();
  const bob = new AptosAccount();

  console.log("\n=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);
  console.log(`Bob: ${bob.address()}`);

  await faucetClient.fundAccount(alice.address(), 5_000);
  await faucetClient.fundAccount(bob.address(), 5_000);

  await new Promise<void>((resolve) => {
    readline.question(
      "Update the CoinType module with Alice's address, build, copy to the provided path, and press enter.",
      () => {
        resolve();
        readline.close();
      },
    );
  });
  const modulePath = process.argv[2];
  const moduleHex = fs.readFileSync(modulePath).toString("hex");

  console.log("Publishing MoonCoinType module...");
  let txHash = await publishModule(alice, moduleHex);
  await client.waitForTransaction(txHash);

  console.log("Alice will initialize the new coin");
  txHash = await initializeCoin(alice, alice.address());
  await client.waitForTransaction(txHash);

  console.log("Bob registers the newly created coin so he can receive it from Alice");
  txHash = await registerCoin(bob, alice.address());
  await client.waitForTransaction(txHash);
  console.log(`Bob's initial balance: ${await getBalance(bob.address(), alice.address())}`);

  console.log("Alice mints Bob some of the new coin");
  txHash = await mintCoin(alice, alice.address(), bob.address(), 100);
  await client.waitForTransaction(txHash);
  console.log(`Bob's new balance: ${await getBalance(bob.address(), alice.address())}`);
}

if (require.main === module) {
  main().then((resp) => console.log(resp));
}

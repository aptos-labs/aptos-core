// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

export const NODE_URL = "https://fullnode.devnet.aptoslabs.com";
export const FAUCET_URL = "https://faucet.devnet.aptoslabs.com";

//:!:>section_1

/** AptosAccount provides methods around addresses, key-pairs */
import { AptosAccount, TxnBuilderTypes, BCS, MaybeHexString } from "aptos";

//<:!:section_1

//:!:>section_2
/** Wrappers around the Aptos Node and Faucet API */
import { AptosClient, FaucetClient } from "aptos";

//<:!:section_2
//:!:>section_3
const client = new AptosClient(NODE_URL);
/**
 * https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html#getAccount
 * returns the sequence number and authentication key for an account
 *
 * https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html#getAccountResource
 * returns all resources associated with the account
 */

//<:!:section_3

//:!:>section_4
/**
 * https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html#generateBCSTransaction
 * signs a raw transaction, which can be submitted to the blockchain.
 */

/**
 * https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html#submitSignedBCSTransaction
 * submits a signed transaction to the blockchain.
 */

//<:!:section_4
//:!:>section_5
/** Helper method returns the coin balance associated with the account */
export async function accountBalance(accountAddress: MaybeHexString): Promise<number | null> {
  const resource = await client.getAccountResource(accountAddress, "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>");
  if (resource == null) {
    return null;
  }

  return parseInt((resource.data as any)["coin"]["value"]);
}

/**
 * Transfers a given coin amount from a given accountFrom to the recipient's account address.
 * Returns the transaction hash of the transaction used to transfer.
 */
async function transfer(accountFrom: AptosAccount, recipient: MaybeHexString, amount: number): Promise<string> {
  const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString("0x1::aptos_coin::AptosCoin"));

  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x1::coin",
      "transfer",
      [token],
      [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(recipient)), BCS.bcsSerializeUint64(amount)],
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

//<:!:section_5
//:!:>section_6
/** Faucet creates and funds accounts. */
const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

//<:!:section_6
//:!:>section_7
/** run our demo! */
async function main() {
  // Create two accounts, Alice and Bob, and fund Alice but not Bob
  const alice = new AptosAccount();
  const bob = new AptosAccount();

  console.log("\n=== Addresses ===");
  console.log(`Alice: ${alice.address()}`);
  console.log(`Bob: ${bob.address()}`);

  await faucetClient.fundAccount(alice.address(), 5_000);
  await faucetClient.fundAccount(bob.address(), 0);

  console.log("\n=== Initial Balances ===");
  console.log(`Alice: ${await accountBalance(alice.address())}`);
  console.log(`Bob: ${await accountBalance(bob.address())}`);

  // Have Alice give Bob 1000 coins
  const txHash = await transfer(alice, bob.address(), 1_000);
  await client.waitForTransaction(txHash);

  console.log("\n=== Final Balances ===");
  console.log(`Alice: ${await accountBalance(alice.address())}`);
  console.log(`Bob: ${await accountBalance(bob.address())}`);
}

if (require.main === module) {
  main();
}
//<:!:section_7

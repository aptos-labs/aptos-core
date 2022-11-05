/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosClient, AptosAccount, FaucetClient, BCS, TxnBuilderTypes, Types, ApiError } from "aptos";
import { aptosCoinStore } from "../common";
import assert from "assert";
import { FastTransactionClient } from "./fast_transaction_client";

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

const BATCH_SIZE = 50;

const { TypeTagStruct, EntryFunction, StructTag, TransactionPayloadEntryFunction } = TxnBuilderTypes;

const client = new AptosClient(NODE_URL);
const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

export async function sleep(timeMs: number): Promise<null> {
  return new Promise((resolve) => {
    setTimeout(resolve, timeMs);
  });
}

export async function batchSubmission(numTxnsToSubmit: number) {
  // Generates key pair for a new account
  const account1 = new AptosAccount();
  await faucetClient.fundAccount(account1.address(), 100_000_000);

  let resources = await client.getAccountResources(account1.address());
  let accountResource = resources.find((r) => r.type === aptosCoinStore);
  let balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 100_000_000);
  console.log(`account2 coins: ${balance}. Should be 100000000!`);

  const account2 = new AptosAccount();
  // Creates the second account and fund the account with 0 AptosCoin
  await faucetClient.fundAccount(account2.address(), 0);
  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 0);
  console.log(`account2 coins: ${balance}. Should be 0!`);

  const token = new TypeTagStruct(StructTag.fromString("0x1::aptos_coin::AptosCoin"));

  const transferCoinPayload = new TransactionPayloadEntryFunction(
    EntryFunction.natural(
      // Fully qualified module name, `AccountAddress::ModuleName`
      "0x1::coin",
      // Module function
      "transfer",
      // The coin type to transfer
      [token],
      // Arguments for function `transfer`: receiver account address and amount to transfer
      [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account2.address())), BCS.bcsSerializeUint64(1)],
    ),
  );

  const fastClient = new FastTransactionClient(client);
  fastClient.subscribeTTLedTxn((txn) => {
    console.error(`Transaction with hash ${txn.hash} ttled.`);
  });

  const numBatches = numTxnsToSubmit / BATCH_SIZE;

  let latestPendTxnHash = "";

  for (let i = 0; i < numBatches; i += 1) {
    console.log(`Submitting batch no. ${i + 1}`);
    const pendingTxnsPromises: Promise<Types.PendingTransaction>[] = [];
    for (let i = 0; i < BATCH_SIZE; i += 1) {
      const pendingTxn = fastClient.submitTxn(account1, transferCoinPayload, {
        maxGasAmount: BigInt(200000),
        gasUnitPrice: BigInt(100),
      });
      pendingTxnsPromises.push(pendingTxn);
    }

    await Promise.allSettled(pendingTxnsPromises);

    let backPressure = false;
    for (let i = 0; i < pendingTxnsPromises.length; i += 1) {
      try {
        const pendingTxn = await pendingTxnsPromises[i];
        latestPendTxnHash = pendingTxn.hash;
        console.log(latestPendTxnHash);
      } catch (e) {
        if (e instanceof ApiError && e.errorCode === "mempool_is_full") {
          console.error(`mempool is full. ${e.message}`);

          backPressure = true;
        } else {
          // Unexpected error
          throw e;
        }
      }
    }

    if (backPressure) {
      console.log("Slowing down...");
      // Slow down
      await sleep(10 * 1000);
    }
  }

  await client.waitForTransaction(latestPendTxnHash);

  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  console.log(`account2 final coins: ${balance}.`);
}

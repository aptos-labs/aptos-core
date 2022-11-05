/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosClient, AptosAccount, FaucetClient, BCS, TxnBuilderTypes, Types } from "aptos";
import { aptosCoinStore } from "../common";
import assert from "assert";
import { FastTransactionClient } from "./fast_transaction_client";

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

const { TypeTagStruct, EntryFunction, StructTag, TransactionPayloadEntryFunction } = TxnBuilderTypes;

const client = new AptosClient(NODE_URL);
const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

function getFastClient(): FastTransactionClient {
  const fastClient = new FastTransactionClient(client);
  fastClient.subscribeTTLedTxn((txn) => {
    console.error(`Transaction with hash ${txn.hash} ttled.`);
  });
  return fastClient;
}

export async function sleep(timeMs: number): Promise<null> {
  return new Promise((resolve) => {
    setTimeout(resolve, timeMs);
  });
}

export async function multiClientSubmission(numTxnsToSubmit: number, numClients: number) {
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

  const arr = [...Array(numClients).keys()];

  const fastClients = arr.map((_) => getFastClient());

  const numTxnsArray = arr.map((_) => Math.ceil(numTxnsToSubmit / numClients));

  let lastestPendingTxn: Types.PendingTransaction;
  async function sendTxns(fastClient: FastTransactionClient, numTxns: number) {
    for (let i = 0; i < numTxns; i += 1) {
      const pendingTxn = await fastClient.submitTxn(account1, transferCoinPayload, {
        maxGasAmount: BigInt(200000) + BigInt(Math.floor(Math.random() * 100)),
        gasUnitPrice: BigInt(100) + BigInt(Math.floor(Math.random() * 100)),
      });

      if (!lastestPendingTxn || BigInt(pendingTxn.sequence_number) > BigInt(lastestPendingTxn.sequence_number)) {
        lastestPendingTxn = pendingTxn;
      }

      console.log(pendingTxn.hash);

      await sleep(Math.floor(Math.random() * 150));
    }
  }

  await Promise.allSettled(arr.map((i) => sendTxns(fastClients[i], numTxnsArray[i])));

  await client.waitForTransaction(lastestPendingTxn!.hash);

  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  console.log(`account2 final coins: ${balance}`);
}

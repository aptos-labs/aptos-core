/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosClient, AptosAccount, FaucetClient, BCS, TxnBuilderTypes } from "aptos";
import { aptosCoinStore } from "./common";
import assert from "assert";

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

const {
  AccountAddress,
  TypeTagStruct,
  EntryFunction,
  StructTag,
  TransactionPayloadEntryFunction,
  RawTransaction,
  ChainId,
} = TxnBuilderTypes;

/**
 * This code example demonstrates the process of moving test coins from one account to another.
 */
(async () => {
  const client = new AptosClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

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

  // TS SDK support 3 types of transaction payloads: `EntryFunction`, `Script` and `Module`.
  // See https://aptos-labs.github.io/ts-sdk-doc/ for the details.
  const entryFunctionPayload = new TransactionPayloadEntryFunction(
    EntryFunction.natural(
      // Fully qualified module name, `AccountAddress::ModuleName`
      "0x1::coin",
      // Module function
      "transfer",
      // The coin type to transfer
      [token],
      // Arguments for function `transfer`: receiver account address and amount to transfer
      [BCS.bcsToBytes(AccountAddress.fromHex(account2.address())), BCS.bcsSerializeUint64(717)],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(account1.address()),
    client.getChainId(),
  ]);

  // See class definiton here
  // https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.RawTransaction.html#constructor.
  const rawTxn = new RawTransaction(
    // Transaction sender account address
    AccountAddress.fromHex(account1.address()),
    BigInt(sequenceNumber),
    entryFunctionPayload,
    // Max gas unit to spend
    BigInt(2000),
    // Gas price per unit
    BigInt(100),
    // Expiration timestamp. Transaction is discarded if it is not executed within 10 seconds from now.
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new ChainId(chainId),
  );

  // Sign the raw transaction with account1's private key
  const bcsTxn = AptosClient.generateBCSTransaction(account1, rawTxn);

  const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);

  await client.waitForTransaction(transactionRes.hash);

  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoinStore);
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 717);
  console.log(`account2 coins: ${balance}. Should be 717!`);
})();

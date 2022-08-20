/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import { AptosClient, AptosAccount, FaucetClient, Types } from "aptos";
import { aptosCoin } from "./constants";

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

console.log("Node URL", NODE_URL);
console.log("Faucet URL", FAUCET_URL);

(async () => {
  const client = new AptosClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

  const account1 = new AptosAccount();
  await faucetClient.fundAccount(account1.address(), 100000);
  let resources = await client.getAccountResources(account1.address());
  let accountResource = resources.find((r) => r.type === aptosCoin)!;
  let balance = (accountResource.data as { coin: { value: string } }).coin.value;
  console.log(`account1 coins: ${balance}. Should be 100000!`);

  const account2 = new AptosAccount();
  await faucetClient.fundAccount(account2.address(), 0);
  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoin)!;
  balance = (accountResource.data as { coin: { value: string } }).coin.value;
  console.log(`account2 coins: ${balance}. Should be 0!`);

  const payload: Types.TransactionPayload = {
    type: "entry_function_payload",
    function: "0x1::coin::transfer",
    type_arguments: ["0x1::aptos_coin::AptosCoin"],
    arguments: [account2.address().hex(), 717],
  };
  const txnRequest = await client.generateTransaction(account1.address(), payload);
  const signedTxn = await client.signTransaction(account1, txnRequest);
  const transactionRes = await client.submitTransaction(signedTxn);
  await client.waitForTransaction(transactionRes.hash);

  resources = await client.getAccountResources(account2.address());
  accountResource = resources.find((r) => r.type === aptosCoin)!;
  balance = (accountResource.data as { coin: { value: string } }).coin.value;
  console.log(`account2 coins: ${balance}. Should be 717!`);
})();

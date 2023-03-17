/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();
import { AptosAccount, TxnBuilderTypes, OptionalTransactionArgs, AptosClient, FaucetClient, BCS } from "aptos";
import { exit } from "process";
import { BatchTransaction } from "./batch_transactions";

export type Transaction = {
  sender: AptosAccount;
  payload: TxnBuilderTypes.TransactionPayload;
  extraArgs?: OptionalTransactionArgs;
};

async function main() {
  const client = new AptosClient("https://fullnode.devnet.aptoslabs.com");
  //const faucetClient = new FaucetClient("http://0.0.0.0:8080/v1", "http://0.0.0.0:8081");
  const faucetClient = new FaucetClient(
    "https://fullnode.devnet.aptoslabs.com",
    "https://faucet.devnet.aptoslabs.com",
    { TOKEN: "klsdjfoids6f78f3nm2wuvnaslku6y2387hasdfuph32vclkjdf8jsdfj2983fjafp12jfjfj0p00y93378" },
  );

  const account1 = new AptosAccount();
  await faucetClient.fundAccount(account1.address(), 100_000_000);
  const account2 = new AptosAccount();

  const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      "0x1::aptos_account",
      "transfer",
      [],
      [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account2.address())), BCS.bcsSerializeUint64(1)],
    ),
  );

  console.log("/////submiting batch transactions for account", account1.address().hex());

  const transactions: Transaction[] = [];

  for (let i = 0; i < 5; i++) {
    transactions.push({
      sender: account1,
      payload: entryFunctionPayload,
      extraArgs: { maxGasAmount: BigInt(200000), gasUnitPrice: BigInt(100) },
    });
  }

  const batch = new BatchTransaction();
  const result = await batch.send(transactions);
  console.log(result);
  exit(0);
}

main();

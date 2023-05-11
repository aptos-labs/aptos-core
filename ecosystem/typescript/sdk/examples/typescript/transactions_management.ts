/**
 * This example demonstrates how a client can utilize the TransactionWorker class.
 */

import { AptosAccount, BCS, TxnBuilderTypes, TransactionWorker, FaucetClient, Provider, Types } from "aptos";
import { exit } from "process";
import { NODE_URL, FAUCET_URL } from "./common";

const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });

const faucet = new FaucetClient(NODE_URL, FAUCET_URL);

async function main() {
  const accountsCount = 5;
  const transactionsCount = 100;
  const totalTransactions = accountsCount * transactionsCount;

  console.log("starting...");
  console.log(new Date().toTimeString());
  // create senders and recipients accounts
  const senders: AptosAccount[] = [];
  const recipients: AptosAccount[] = [];
  for (let i = 0; i < accountsCount; i++) {
    senders.push(new AptosAccount());
    recipients.push(new AptosAccount());
  }
  console.log(`${senders.length * 2} sender and recipient accounts created`);

  // funds sender accounts
  const funds: Array<Promise<string[]>> = [];

  for (let i = 0; i < senders.length; i++) {
    funds.push(faucet.fundAccount(senders[i].address().noPrefix(), 10000000000));
  }

  // send requests
  await Promise.all(funds);
  console.log(`${funds.length} sender accounts funded`);
  for (const acc in senders) {
    const curr = senders[acc] as AptosAccount;
    console.log(curr.address().hex());
  }

  // read sender accounts
  const balances: Array<Promise<Types.AccountData>> = [];
  for (let i = 0; i < senders.length; i++) {
    balances.push(provider.getAccount(senders[i].address().hex()));
  }
  // send requests
  await Promise.all(balances);

  //await Promise.all(balances);
  console.log(`${balances.length} sender account balances checked`);

  // create transactions
  const payloads: any[] = [];
  // 100 transactions
  for (let j = 0; j < transactionsCount; j++) {
    // 5 recipients
    for (let i = 0; i < recipients.length; i++) {
      const txn = new TxnBuilderTypes.TransactionPayloadEntryFunction(
        TxnBuilderTypes.EntryFunction.natural(
          "0x1::aptos_account",
          "transfer",
          [],
          [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(recipients[i].address())), BCS.bcsSerializeUint64(5)],
        ),
      );
      payloads.push(txn);
    }
  }

  const batchTransactions = (payloads: any[], sender: AptosAccount) => {
    const transactionWorker = new TransactionWorker(provider, sender);
    const waitFor: Array<Promise<void>> = [];

    transactionWorker.start();

    transactionWorker.on("transactionsFulfilled", async (data) => {
      /**
       * data is an array with 2 elements
       * data[0] = the amount of processed transactions
       * data[1] = the hash value of the processed transaction
       */
      waitFor.push(provider.waitForTransaction(data[1], { checkSuccess: true }));
      // all expected transactions have been fulfilled
      if (data[0] === totalTransactions) {
        await Promise.all(waitFor);
        console.log("transactions submitted");
        console.log(new Date().toTimeString());
        await checkAccounts();
      }
    });

    // push transactions to queue
    for (const payload in payloads) {
      transactionWorker.push(payloads[payload]);
    }
  };

  for (let i = 0; i < senders.length; i++) {
    batchTransactions(payloads, senders[i]);
  }

  // check for account's sequence numbers
  const checkAccounts = async () => {
    const waitFor: Array<Promise<Types.AccountData>> = [];
    for (let i = 0; i < senders.length; i++) {
      waitFor.push(provider.getAccount(senders[i].address()));
    }

    const res = await Promise.all(waitFor);
    console.log(`transactions verified`);
    console.log(new Date().toTimeString());
    for (const account in res) {
      const currentAccount = res[account] as Types.AccountData;
      console.log(
        `sender account ${currentAccount.authentication_key} final sequence number is ${currentAccount.sequence_number}`,
      );
    }
    exit(0);
  };
}

async function sleep(ms: number): Promise<void> {
  return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

main();

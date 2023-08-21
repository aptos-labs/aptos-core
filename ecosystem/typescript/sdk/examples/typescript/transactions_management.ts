/**
 * This example demonstrates how a client can utilize the TransactionWorker class.
 *
 * The TransactionWorker provides a simple framework for receiving payloads to be processed. It
 * acquires an account new sequence number, produces a signed transaction and
 * then submits the transaction. In other tasks, it waits for resolution of the submission
 * process or get pre-execution validation error and waits for the resolution of the execution process
 * or get an execution validation error.
 *
 * The TransactionWorker constructor accepts
 * @param provider - a client provider
 * @param sender - the sender account: AptosAccount
 * @param maxWaitTime - the max wait time to wait before restarting the local sequence number to the current on-chain state
 * @param maximumInFlight - submit up to `maximumInFlight` transactions per account
 * @param sleepTime - If `maximumInFlight` are in flight, wait `sleepTime` seconds before re-evaluating
 *
 * Read more about it here {@link https://aptos.dev/guides/transaction-management}
 */

import {
  AptosAccount,
  BCS,
  TxnBuilderTypes,
  TransactionWorker,
  TransactionWorkerEvents,
  FaucetClient,
  Provider,
  Types,
} from "aptos";
import { exit } from "process";
import { NODE_URL, FAUCET_URL } from "./common";

const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });

const faucet = new FaucetClient(NODE_URL, FAUCET_URL);

async function main() {
  const accountsCount = 5;
  const transactionsCount = 100;
  const totalTransactions = accountsCount * transactionsCount;

  const start = Date.now() / 1000; // current time in seconds

  console.log("starting...");
  // create senders and recipients accounts
  const senders: AptosAccount[] = [];
  const recipients: AptosAccount[] = [];
  for (let i = 0; i < accountsCount; i++) {
    senders.push(new AptosAccount());
    recipients.push(new AptosAccount());
  }
  let last = Date.now() / 1000;
  console.log(
    `${senders.length} sender accounts and ${recipients.length} recipient accounts created in ${last - start} seconds`,
  );

  // fund sender accounts
  const funds: Array<Promise<string[]>> = [];

  for (let i = 0; i < senders.length; i++) {
    funds.push(faucet.fundAccount(senders[i].address().noPrefix(), 10000000000));
  }

  await Promise.all(funds);

  console.log(`${funds.length} sender accounts funded in ${Date.now() / 1000 - last} seconds`);
  last = Date.now() / 1000;

  // read sender accounts
  const balances: Array<Promise<Types.AccountData>> = [];
  for (let i = 0; i < senders.length; i++) {
    balances.push(provider.getAccount(senders[i].address().hex()));
  }
  await Promise.all(balances);

  console.log(`${balances.length} sender account balances checked in ${Date.now() / 1000 - last} seconds`);
  last = Date.now() / 1000;

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

  console.log(`sends ${totalTransactions} transactions to chain....`);
  // emit batch transactions
  const promises = senders.map((sender) => batchTransactions(payloads, sender));
  await Promise.all(promises);

  async function batchTransactions(payloads: TxnBuilderTypes.Transaction[], sender: AptosAccount) {
    const transactionWorker = new TransactionWorker(provider, sender);

    transactionWorker.start();

    registerToWorkerEvents(transactionWorker);

    // push transactions to worker queue
    for (const payload in payloads) {
      await transactionWorker.push(payloads[payload]);
    }
  }

  function registerToWorkerEvents(transactionWorker: TransactionWorker) {
    /**
     * The callback from an event listener, i.e `data`, is an array with 2 elements
     * data[0] - the amount of processed transactions
     * data[1] -
     * on a success event, is the hash value of the processed transaction
     * on a failure event, is the reason for the failure
     */
    transactionWorker.on(TransactionWorkerEvents.TransactionSent, async (data) => {
      // all expected transactions have been sent
      if (data[0] === totalTransactions) {
        console.log(`transactions sent in ${Date.now() / 1000 - last} seconds`);
      }
    });

    transactionWorker.on(TransactionWorkerEvents.TransactionSendFailed, async (data) => {
      /**
       * transaction sent failed, up to the user to decide next steps.
       * whether to stop the worker by transactionWorker.stop() and handle
       * the error, or simply return the error to the end user.
       * At this point, we have the failed transaction queue number
       * and the transaction failure reason
       */
      console.log("sentFailed", data);
    });

    transactionWorker.on(TransactionWorkerEvents.TransactionExecuted, async (data) => {
      // all expected transactions have been executed
      if (data[0] === totalTransactions) {
        console.log(`transactions executed in ${Date.now() / 1000 - last} seconds`);
        await checkAccounts();
      }
    });

    transactionWorker.on(TransactionWorkerEvents.TransactionExecutionFailed, async (data) => {
      /**
       * transaction execution failed, up to the user to decide next steps.
       * whether to stop the worker by transactionWorker.stop() and handle
       * the error, or simply return the error to the end user.
       * At this point, we have the failed transaction queue number
       * and the transaction object data
       */
      console.log("executionFailed", data);
    });
  }

  // check for account's sequence numbers
  async function checkAccounts(): Promise<void> {
    const waitFor: Array<Promise<Types.AccountData>> = [];
    for (let i = 0; i < senders.length; i++) {
      waitFor.push(provider.getAccount(senders[i].address()));
    }
    const res = await Promise.all(waitFor);
    console.log(`transactions verified in  ${Date.now() / 1000 - last}  seconds`);
    for (const account in res) {
      const currentAccount = res[account] as Types.AccountData;
      console.log(
        `sender account ${currentAccount.authentication_key} final sequence number is ${currentAccount.sequence_number}`,
      );
    }
    exit(0);
  }
}

main();

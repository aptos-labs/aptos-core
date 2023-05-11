import EventEmitter from "eventemitter3";
import { AptosAccount } from "../account";
import { PendingTransaction } from "../generated";
import { AptosClient, Provider } from "../providers";
import { TxnBuilderTypes } from "../transaction_builder";
import { AccountSequenceNumber } from "./account_sequence_number";

export class TransactionWorker extends EventEmitter {
  readonly provider: Provider;

  readonly account: AptosAccount;

  readonly accountSequnceNumber: AccountSequenceNumber;

  started: boolean;

  stopped: boolean;

  outstandingTransactions: Array<[Promise<PendingTransaction>, bigint]> = [];

  processedTransactions: Array<[string, bigint, any]> = [];

  // TODO support entry function payload from ABI builder
  transactionsQueue: Array<TxnBuilderTypes.TransactionPayload> = [];

  constructor(provider: Provider, account: AptosAccount) {
    super();
    this.provider = provider;
    this.account = account;
    this.started = false;
    this.stopped = false;
    this.accountSequnceNumber = new AccountSequenceNumber(provider, account);
  }

  async submitTransactions() {
    try {
      if (this.transactionsQueue.length === 0) return;
      const sequenceNumber = await this.accountSequnceNumber.nextSequenceNumber();
      if (sequenceNumber === null) return;
      const transaction = await this.generateNextTransaction(this.account, sequenceNumber);
      if (!transaction) return;
      const pendingTransaction = this.provider.submitSignedBCSTransaction(transaction);
      this.outstandingTransactions.push([pendingTransaction, sequenceNumber]);
    } catch (error: any) {
      throw new Error(error);
    }
  }

  async processTransactions() {
    try {
      const awaitingTransactions = [];
      const awaitingSequenceNumbers = [];

      while (this.outstandingTransactions.length > 0) {
        const [pendingTransaction, sequenceNumber] = this.outstandingTransactions.shift()!;

        awaitingTransactions.push(pendingTransaction);
        awaitingSequenceNumbers.push(sequenceNumber);
      }

      try {
        const outputs = await Promise.allSettled(awaitingTransactions);
        for (let i = 0; i < outputs.length && i < awaitingSequenceNumbers.length; i += 1) {
          const output = outputs[i];
          const sequenceNumber = awaitingSequenceNumbers[i];

          if (output.status === "fulfilled") {
            this.processedTransactions.push([output.value.hash, sequenceNumber, null]);
            this.emit("transactionsFulfilled", [this.processedTransactions.length, output.value.hash]);
          } else {
            this.processedTransactions.push([output.status, sequenceNumber, output.reason]);
          }
        }
      } catch (error: any) {
        throw new Error(error);
      }
    } catch (error: any) {
      throw new Error(error);
    }
  }

  async push(payload: TxnBuilderTypes.TransactionPayload): Promise<void> {
    await this.transactionsQueue.push(payload);
  }

  async generateNextTransaction(account: AptosAccount, sequenceNumber: bigint): Promise<Uint8Array | undefined> {
    if (this.transactionsQueue.length === 0) return undefined;
    const payload = await this.transactionsQueue.shift()!;
    const rawTransaction = await this.provider.generateRawTransaction(account.address(), payload, {
      providedSequenceNumber: sequenceNumber,
    });
    const signedTransaction = await AptosClient.generateBCSTransaction(account, rawTransaction);
    return signedTransaction;
  }

  async runTransactions() {
    try {
      while (!this.stopped) {
        /* eslint-disable no-await-in-loop, no-promise-executor-return */
        await Promise.all([this.submitTransactions(), this.processTransactions()]);
        /** 
         * since runTransactions() function runs continuously in a loop, it prevents the execution 
         * from reaching a callback function (e.g when client wants to gracefuly stop the worker). 
         * Add a small delay between iterations to allow other code to run
        /* eslint-disable no-await-in-loop */
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    } catch (error: any) {
      throw new Error(error);
    }
  }

  start() {
    if (this.started) {
      throw new Error("worker has already started");
    }
    this.started = true;
    this.stopped = false;
    this.runTransactions();
  }

  stop() {
    if (this.stopped) {
      throw new Error("worker has already stopped");
    }
    this.stopped = true;
    this.started = false;
  }
}

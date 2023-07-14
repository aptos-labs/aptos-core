import EventEmitter from "eventemitter3";
import { AptosAccount } from "../account";
import { PendingTransaction } from "../generated";
import { AptosClient, Provider } from "../providers";
import { TxnBuilderTypes } from "../transaction_builder";
import { AccountSequenceNumber } from "./account_sequence_number";

export class TransactionWorker extends EventEmitter {
  readonly provider: Provider;

  readonly account: AptosAccount;

  // current account sequence number
  readonly accountSequnceNumber: AccountSequenceNumber;

  // process has started
  started: boolean;

  // process has stopped
  stopped: boolean;

  // transactions payloads waiting to be generated and signed
  // TODO support entry function payload from ABI builder
  transactionsQueue: Array<TxnBuilderTypes.TransactionPayload> = [];

  // signed transactions waiting to be submitted
  outstandingTransactions: Array<[Promise<PendingTransaction>, bigint]> = [];

  // transactions that have been submitted to chain
  processedTransactions: Array<[string, bigint, any]> = [];

  constructor(provider: Provider, account: AptosAccount) {
    super();
    this.provider = provider;
    this.account = account;
    this.started = false;
    this.stopped = false;
    this.accountSequnceNumber = new AccountSequenceNumber(provider, account);
  }

  /**
   * Gets the current account sequence number,
   * generates the transaction with the account sequence number,
   * adds the transaction to the outstanding transaction queue
   * to be processed later.
   */
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

  /**
   * Reads the outstanding transaction queue and submits the transaction to chain.
   *
   * If the transaction has fulfilled, it pushes the transaction to the processed
   * transactions queue and fires a transactionsFulfilled event.
   *
   * If the transaction has failed, it pushes the transaction to the processed
   * transactions queue with the failure reason and fires a transactionsFailed event.
   */
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

  /**
   * Push transaction to the transactions queue
   * @param payload Transaction payload
   */
  async push(payload: TxnBuilderTypes.TransactionPayload): Promise<void> {
    await this.transactionsQueue.push(payload);
  }

  /**
   * Generates a signed transaction that can be submitted to chain
   * @param account an Aptos account
   * @param sequenceNumber a sequence number the transaction will be generated with
   * @returns
   */
  async generateNextTransaction(account: AptosAccount, sequenceNumber: bigint): Promise<Uint8Array | undefined> {
    if (this.transactionsQueue.length === 0) return undefined;
    const payload = await this.transactionsQueue.shift()!;
    const rawTransaction = await this.provider.generateRawTransaction(account.address(), payload, {
      providedSequenceNumber: sequenceNumber,
    });
    const signedTransaction = await AptosClient.generateBCSTransaction(account, rawTransaction);
    return signedTransaction;
  }

  /**
   * Starts transaction submission and transaction processing.
   */
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

  /**
   * Starts the transaction management process.
   */
  start() {
    if (this.started) {
      throw new Error("worker has already started");
    }
    this.started = true;
    this.stopped = false;
    this.runTransactions();
  }

  /**
   * Stops the the transaction management process.
   */
  stop() {
    if (this.stopped) {
      throw new Error("worker has already stopped");
    }
    this.stopped = true;
    this.started = false;
  }
}

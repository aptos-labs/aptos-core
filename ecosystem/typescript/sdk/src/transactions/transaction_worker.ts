/* eslint-disable no-await-in-loop */

/**
 * TransactionWorker provides a simple framework for receiving payloads to be processed.
 *
 * Once one `start()` the process, the worker acquires the current account's next sequence number
 * (by using the AccountSequenceNumber class), generates a signed transaction and pushes an async
 * submission process into a `outstandingTransactions` queue.
 * At the same time, the worker processes transactions by reading the `outstandingTransactions` queue
 * and submits the next transaction to chain, it
 * 1) waits for resolution of the submission process or get pre-execution validation error
 * and 2) waits for the resolution of the execution process or get an execution error.
 * The worker fires events for any submission and/or execution success and/or failure.
 */

import EventEmitter from "eventemitter3";
import { AptosAccount } from "../account";
import { PendingTransaction, Transaction } from "../generated";
import { AptosClient, Provider } from "../providers";
import { TxnBuilderTypes } from "../transaction_builder";
import { AccountSequenceNumber } from "./account_sequence_number";
import { AsyncQueue, AsyncQueueCancelledError } from "./async_queue";

const promiseFulfilledStatus = "fulfilled";

// Events
const transactionSent = "transactionSent";
const sentFailed = "sentFailed";

const transactionExecuted = "transactionExecuted";
const executionFailed = "executionFailed";
export class TransactionWorker extends EventEmitter {
  readonly provider: Provider;

  readonly account: AptosAccount;

  // current account sequence number
  readonly accountSequnceNumber: AccountSequenceNumber;

  readonly taskQueue: AsyncQueue<() => Promise<void>> = new AsyncQueue<() => Promise<void>>();

  // process has started
  started: boolean;

  // transactions payloads waiting to be generated and signed
  // TODO support entry function payload from ABI builder
  transactionsQueue = new AsyncQueue<TxnBuilderTypes.TransactionPayload>();

  // signed transactions waiting to be submitted
  outstandingTransactions = new AsyncQueue<[Promise<PendingTransaction>, bigint]>();

  // transactions that have been submitted to chain
  sentTransactions: Array<[string, bigint, any]> = [];

  // transactions that have been committed to chain
  executedTransactions: Array<[string, bigint, any]> = [];

  /**
   * Provides a simple framework for receiving payloads to be processed.
   *
   * @param provider - a client provider
   * @param sender - a sender as AptosAccount
   * @param maxWaitTime - the max wait time to wait before resyncing the sequence number to the current on-chain state
   * @param maximumInFlight - submit up to `maximumInFlight` transactions per account
   * @param sleepTime - If `maximumInFlight` are in flight, wait `sleepTime` seconds before re-evaluating
   */
  constructor(
    provider: Provider,
    account: AptosAccount,
    maxWaitTime: number,
    maximumInFlight: number,
    sleepTime: number,
  ) {
    super();
    this.provider = provider;
    this.account = account;
    this.started = false;
    this.accountSequnceNumber = new AccountSequenceNumber(provider, account, maxWaitTime, maximumInFlight, sleepTime);
  }

  /**
   * Gets the current account sequence number,
   * generates the transaction with the account sequence number,
   * adds the transaction to the outstanding transaction queue
   * to be processed later.
   */
  async submitNextTransaction() {
    try {
      /* eslint-disable no-constant-condition */
      while (true) {
        if (this.transactionsQueue.isEmpty()) return;
        const sequenceNumber = await this.accountSequnceNumber.nextSequenceNumber();
        if (sequenceNumber === null) return;
        const transaction = await this.generateNextTransaction(this.account, sequenceNumber);
        if (!transaction) return;
        const pendingTransaction = this.provider.submitSignedBCSTransaction(transaction);
        await this.outstandingTransactions.enqueue([pendingTransaction, sequenceNumber]);
      }
    } catch (error: any) {
      if (error instanceof AsyncQueueCancelledError) {
        return;
      }
      console.log(error);
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
      /* eslint-disable no-constant-condition */
      while (true) {
        const awaitingTransactions = [];
        const sequenceNumbers = [];
        let [pendingTransaction, sequenceNumber] = await this.outstandingTransactions.dequeue();

        awaitingTransactions.push(pendingTransaction);
        sequenceNumbers.push(sequenceNumber);

        while (!this.outstandingTransactions.isEmpty()) {
          [pendingTransaction, sequenceNumber] = await this.outstandingTransactions.dequeue();

          awaitingTransactions.push(pendingTransaction);
          sequenceNumbers.push(sequenceNumber);
        }
        // send awaiting transactions to chain
        const sentTransactions = await Promise.allSettled(awaitingTransactions);
        for (let i = 0; i < sentTransactions.length && i < sequenceNumbers.length; i += 1) {
          // check sent transaction status
          const sentTransaction = sentTransactions[i];
          sequenceNumber = sequenceNumbers[i];
          if (sentTransaction.status === promiseFulfilledStatus) {
            // transaction sent to chain
            this.sentTransactions.push([sentTransaction.value.hash, sequenceNumber, null]);
            this.emit(transactionSent, [this.sentTransactions.length, sentTransaction.value.hash]);
            // check sent transaction execution
            await this.checkTransaction(sentTransaction, sequenceNumber);
          } else {
            // send transaction failed
            this.sentTransactions.push([sentTransaction.status, sequenceNumber, sentTransaction.reason]);
            this.emit(sentFailed, [this.sentTransactions.length, sentTransaction.reason]);
          }
        }
      }
    } catch (error: any) {
      if (error instanceof AsyncQueueCancelledError) {
        return;
      }
      console.log(error);
    }
  }

  /**
   * Once transaction has been sent to chain, we check for its execution status.
   * @param sentTransaction transactions that were sent to chain and are now waiting to be executed
   * @param sequenceNumber the account's sequence number that was sent with the transaction
   */
  async checkTransaction(sentTransaction: PromiseFulfilledResult<PendingTransaction>, sequenceNumber: bigint) {
    const waitFor: Array<Promise<Transaction>> = [];
    waitFor.push(this.provider.waitForTransactionWithResult(sentTransaction.value.hash, { checkSuccess: true }));
    const sentTransactions = await Promise.allSettled(waitFor);

    for (let i = 0; i < sentTransactions.length; i += 1) {
      const executedTransaction = sentTransactions[i];
      if (executedTransaction.status === promiseFulfilledStatus) {
        // transaction executed to chain
        this.executedTransactions.push([executedTransaction.value.hash, sequenceNumber, null]);
        this.emit(transactionExecuted, [this.executedTransactions.length, executedTransaction.value.hash]);
      } else {
        // transaction execution failed
        this.executedTransactions.push([executedTransaction.status, sequenceNumber, executedTransaction.reason]);
        this.emit(executionFailed, [this.executedTransactions.length, executedTransaction.reason]);
      }
    }
  }

  /**
   * Push transaction to the transactions queue
   * @param payload Transaction payload
   */
  async push(payload: TxnBuilderTypes.TransactionPayload): Promise<void> {
    await this.transactionsQueue.enqueue(payload);
  }

  /**
   * Generates a signed transaction that can be submitted to chain
   * @param account an Aptos account
   * @param sequenceNumber a sequence number the transaction will be generated with
   * @returns
   */
  async generateNextTransaction(account: AptosAccount, sequenceNumber: bigint): Promise<Uint8Array | undefined> {
    if (this.transactionsQueue.isEmpty()) return undefined;
    const payload = await this.transactionsQueue.dequeue();
    const rawTransaction = await this.provider.generateRawTransaction(account.address(), payload, {
      providedSequenceNumber: sequenceNumber,
    });
    const signedTransaction = AptosClient.generateBCSTransaction(account, rawTransaction);
    return signedTransaction;
  }

  /**
   * Starts transaction submission and transaction processing.
   */
  async run() {
    try {
      while (!this.taskQueue.isCancelled()) {
        const task = await this.taskQueue.dequeue();
        await task();
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
    this.taskQueue.enqueue(() => this.submitNextTransaction());
    this.taskQueue.enqueue(() => this.processTransactions());
    this.run();
  }

  /**
   * Stops the the transaction management process.
   */
  stop() {
    if (this.taskQueue.isCancelled()) {
      throw new Error("worker has already stopped");
    }
    this.started = false;
    this.taskQueue.cancel();
  }
}

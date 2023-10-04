import {
  getGasPriceEstimation,
  getTransactionByHash,
  getTransactionByVersion,
  getTransactions,
  isTransactionPending,
  waitForTransaction,
} from "../internal/transaction";
import { AnyNumber, GasEstimation, PaginationArgs, TransactionResponse } from "../types";
import { AptosConfig } from "./aptos_config";

export class Transaction {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * Queries on-chain transactions. This function will not return pending
   * transactions. For that, use `getTransactionsByHash`.
   *
   * @param options Optional pagination object
   *
   * @returns Array of on-chain transactions
   */
  async getTransactions(args: { options?: PaginationArgs }): Promise<TransactionResponse[]> {
    const transactions = await getTransactions({ aptosConfig: this.config, ...args });
    return transactions;
  }

  /**
   * @param txnVersion - Transaction version is a uint64 number.
   * @returns On-chain transaction. Only on-chain transactions have versions, so this
   * function cannot be used to query pending transactions.
   */
  async getTransactionByVersion(args: { txnVersion: AnyNumber }): Promise<TransactionResponse> {
    const transaction = await getTransactionByVersion({ aptosConfig: this.config, ...args });
    return transaction;
  }

  /**
   * @param txnHash - Transaction hash should be hex-encoded bytes string with 0x prefix.
   * @returns Transaction from mempool (pending) or on-chain (committed) transaction
   */
  async getTransactionByHash(args: { txnHash: string }): Promise<TransactionResponse> {
    const transaction = await getTransactionByHash({ aptosConfig: this.config, ...args });
    return transaction;
  }

  /**
   * Defines if specified transaction is currently in pending state
   * @param txnHash A hash of transaction
   *
   * To create a transaction hash:
   *
   * 1. Create a hash message from the bytes: "Aptos::Transaction" bytes + the BCS-serialized Transaction bytes.
   * 2. Apply hash algorithm SHA3-256 to the hash message bytes.
   * 3. Hex-encode the hash bytes with 0x prefix.
   *
   * @returns `true` if transaction is in pending state and `false` otherwise
   */
  async isPendingTransaction(args: { txnHash: string }): Promise<boolean> {
    const isPending = await isTransactionPending({ aptosConfig: this.config, ...args });
    return isPending;
  }

  /**
   * Wait for a transaction to move past pending state.
   *
   * There are 4 possible outcomes:
   * 1. Transaction is processed and successfully committed to the blockchain.
   * 2. Transaction is rejected for some reason, and is therefore not committed
   *    to the blockchain.
   * 3. Transaction is committed but execution failed, meaning no changes were
   *    written to the blockchain state.
   * 4. Transaction is not processed within the specified timeout.
   *
   * In case 1, this function resolves with the transaction response returned
   * by the API.
   *
   * In case 2, the function will throw an ApiError, likely with an HTTP status
   * code indicating some problem with the request (e.g. 400).
   *
   * In case 3, if `checkSuccess` is false (the default), this function returns
   * the transaction response just like in case 1, in which the `success` field
   * will be false. If `checkSuccess` is true, it will instead throw a
   * FailedTransactionError.
   *
   * In case 4, this function throws a WaitForTransactionError.
   *
   * @param txnHash The hash of a transaction previously submitted to the blockchain.
   * @param extraArgs.timeoutSecs Timeout in seconds. Defaults to 20 seconds.
   * @param extraArgs.checkSuccess See above. Defaults to false.
   * @returns See above.
   *
   * @example
   * ```
   * const rawTransaction = await this.generateRawTransaction(sender.address(), payload, extraArgs);
   * const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTransaction);
   * const pendingTransaction = await this.submitSignedBCSTransaction(bcsTxn);
   * const transasction = await this.aptosClient.waitForTransactionWithResult(pendingTransaction.hash);
   * ```
   */
  async waitForTransaction(args: {
    txnHash: string;
    extraArgs: { timeoutSecs?: number; checkSuccess?: boolean };
  }): Promise<TransactionResponse> {
    const transaction = await waitForTransaction({ aptosConfig: this.config, ...args });
    return transaction;
  }

  /**
   * Gives an estimate of the gas unit price required to get a
   * transaction on chain in a reasonable amount of time.
   * For more information {@link https://fullnode.mainnet.aptoslabs.com/v1/spec#/operations/estimate_gas_price}
   *
   * @returns Object holding the outputs of the estimate gas API
   * @example
   * ```
   * {
   *  gas_estimate: number;
   *  deprioritized_gas_estimate?: number;
   *  prioritized_gas_estimate?: number;
   * }
   * ```
   */
  async getGasPriceEstimation(): Promise<GasEstimation> {
    const gasEstimation = await getGasPriceEstimation({ aptosConfig: this.config });
    return gasEstimation;
  }
}

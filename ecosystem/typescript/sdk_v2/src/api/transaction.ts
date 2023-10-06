import {
  getGasPriceEstimation,
  getTransactionByHash,
  getTransactionByVersion,
  getTransactions,
  isTransactionPending,
  waitForTransaction,
} from "../internal/transaction";
import { AnyNumber, GasEstimation, HexInput, PaginationArgs, TransactionResponse } from "../types";
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
   *
   * To create a transaction hash:
   *
   * 1. Create a hash message from the bytes: "Aptos::Transaction" bytes + the BCS-serialized Transaction bytes.
   * 2. Apply hash algorithm SHA3-256 to the hash message bytes.
   * 3. Hex-encode the hash bytes with 0x prefix.
   *
   * @param txnHash A hash of transaction
   * @returns `true` if transaction is in pending state and `false` otherwise
   */
  async isPendingTransaction(args: { txnHash: HexInput }): Promise<boolean> {
    const isPending = await isTransactionPending({ aptosConfig: this.config, ...args });
    return isPending;
  }

  /**
   * Waits for a transaction to move past the pending state.
   * 
   * There are 4 cases.
   * 1. Transaction is successfully processed and committed to the chain.
   *    - The function will resolve with the transaction response from the API.
   * 2. Transaction is rejected for some reason, and is therefore not committed to the blockchain.
   *    - The function will throw an AptosApiError with an HTTP status code indicating some problem with the request.
   * 3. Transaction is committed but execution failed, meaning no changes were
   *    written to the blockchain state.
   *    - If `checkSuccess` is true, the function will throw a FailedTransactionError
   *      If `checkSuccess` is false, the function will resolve with the transaction response where the `success` field is false.
   * 4. Transaction does not move past the pending state within `extraArgs.timeoutSecs` seconds.
   *    - The function will throw a WaitForTransactionError
   * 
   * 
   * @param txnHash The hash of a transaction previously submitted to the blockchain.
   * @param extraArgs.timeoutSecs Timeout in seconds. Defaults to 20 seconds.
   * @param extraArgs.checkSuccess A boolean which controls whether the function will error if the transaction failed. 
   *   Defaults to true.  See case 3 above.
   * @returns The transaction on-chain.  See above for more details.
   */
  async waitForTransaction(args: {
    txnHash: HexInput;
    extraArgs?: { timeoutSecs?: number; checkSuccess?: boolean };
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

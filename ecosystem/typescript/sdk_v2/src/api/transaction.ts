import { getGasPriceEstimation, getTransactions } from "../internal/transaction";
import { GasEstimation, PaginationArgs, TransactionResponse } from "../types";
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

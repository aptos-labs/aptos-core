// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { get } from "../client/get";
import { AnyNumber, PaginationArgs, TransactionResponse } from "../types";
import { AptosApiType } from "../utils/const";
import { paginateWithCursor } from "../utils/paginate_with_cursor";
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
   * @param aptosConfig Optional pagination object
   * @param options The start transaction version of the page. Default is the latest ledger version
   * @returns Array of on-chain transactions
   */
  async getTransactions(args: {
    aptosConfig: AptosConfig;
    options?: PaginationArgs;
  }): Promise<TransactionResponse[]> {
    const { aptosConfig, options } = args;
    const data = await paginateWithCursor<{}, TransactionResponse[]>({
      url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
      endpoint: "transactions",
      originMethod: "getTransactions",
      params: { start: options?.start, limit: options?.limit },
      overrides: { ...aptosConfig.clientConfig },
    });
    return data;
  }

  /**
   * @param txnHash - Transaction hash should be hex-encoded bytes string with 0x prefix.
   * @returns Transaction from mempool (pend
   * ing) or on-chain (committed) transaction
   */
  async getTransactionByHash(args: {
    txnHash: string;
    aptosConfig: AptosConfig;
  }): Promise<TransactionResponse> {
    const { txnHash, aptosConfig } = args;
    const { data } = await get<{}, TransactionResponse>({
      url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
      endpoint: `transactions/by_hash/${txnHash}`,
      originMethod: "getTransactionByHash",
      overrides: { ...aptosConfig.clientConfig },
    });

    return data;
  }

  /**
   * @param txnVersion - Transaction version is an uint64 number.
   * @returns On-chain transaction. Only on-chain transactions have versions, so this
   * function cannot be used to query pending transactions.
   */
  async getTransactionByVersion(args: {
    txnVersion: AnyNumber;
    aptosConfig: AptosConfig;
  }): Promise<TransactionResponse> {
    const { txnVersion, aptosConfig } = args;
    const { data } = await get<{}, TransactionResponse>({
      url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
      endpoint: `transactions/by_version/${txnVersion}`,
      originMethod: "getTransactionByVersion",
      overrides: { ...aptosConfig.clientConfig },
    });

    return data;
  }
}
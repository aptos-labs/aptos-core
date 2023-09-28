/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/transaction}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * transaction namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptos_config";
import { get } from "../client";
import { GasEstimation, PaginationArgs, TransactionResponse } from "../types";
import { AptosApiType } from "../utils/const";
import { paginateWithCursor } from "../utils/paginate_with_cursor";

export async function getTransactions(args: {
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
  }, aptosConfig);
  return data;
}

export async function getGasPriceEstimation(args: { aptosConfig: AptosConfig }) {
  const { aptosConfig } = args;
  const { data } = await get<{}, GasEstimation>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: "estimate_gas_price",
    originMethod: "getGasPriceEstimation",
    overrides: { ...aptosConfig.clientConfig },
  }, aptosConfig);
  return data;
}

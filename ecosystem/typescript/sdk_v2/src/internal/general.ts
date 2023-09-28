/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/general}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * general namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptos_config";
import {get, getFullNode, post} from "../client";
import { Block, GraphqlQuery, LedgerInfo, LedgerVersion, MoveValue, TableItemRequest, ViewRequest } from "../types";
import { AptosApiType } from "../utils/const";

export async function getLedgerInfo(args: { aptosConfig: AptosConfig }): Promise<LedgerInfo> {
  const { aptosConfig } = args;
  const { data } = await getFullNode<{}, LedgerInfo>(
    {
      aptosConfig,
      name: "getLedgerInfo",
      path: "",
    },
  );
  return data;
}

export async function getBlockByVersion(args: {
  aptosConfig: AptosConfig;
  blockVersion: number;
  options?: { withTransactions?: boolean };
}): Promise<Block> {
  const { aptosConfig, blockVersion, options } = args;
  const { data } = await getFullNode<{}, Block>(
    {
      aptosConfig,
      name: "getBlockByVersion",
      path: `blocks/by_version/${blockVersion}`,
      params: { with_transactions: options?.withTransactions },
    },
  );
  return data;
}

export async function getBlockByHeight(args: {
  aptosConfig: AptosConfig;
  blockHeight: number;
  options?: { withTransactions?: boolean };
}): Promise<Block> {
  const { aptosConfig, blockHeight, options } = args;
  const { data } = await getFullNode<{}, Block>(
    {
      aptosConfig,
      name: "getBlockByHeight",
      path: `blocks/by_height/${blockHeight}`,
      params: { with_transactions: options?.withTransactions },
    },
  );
  return data;
}

export async function getTableItem(args: {
  aptosConfig: AptosConfig;
  handle: string;
  data: TableItemRequest;
  options?: LedgerVersion;
}): Promise<any> {
  const { aptosConfig, handle, data, options } = args;
  const response = await post<TableItemRequest, any>(
    {
      url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
      body: data,
      path: `tables/${handle}/item`,
      name: "getTableItem",
      params: { ledger_version: options?.ledgerVersion },
    },
    aptosConfig,
  );
  return response.data;
}

export async function view(args: {
  aptosConfig: AptosConfig;
  payload: ViewRequest;
  options?: LedgerVersion;
}): Promise<MoveValue[]> {
  const { aptosConfig, payload, options } = args;
  const { data } = await post<ViewRequest, MoveValue[]>(
    {
      url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
      body: payload,
      path: "view",
      name: "view",
      params: { ledger_version: options?.ledgerVersion },
    },
    aptosConfig,
  );
  return data;
}

export async function queryIndexer<T>(args: {
  aptosConfig: AptosConfig;
  query: GraphqlQuery;
  originMethod?: string;
}): Promise<T> {
  const { aptosConfig, query, originMethod } = args;
  const { data } = await post<GraphqlQuery, T>(
    {
      url: aptosConfig.getRequestUrl(AptosApiType.INDEXER),
      body: query,
      name: originMethod ?? "queryIndexer",
      overrides: { WITH_CREDENTIALS: false },
    },
    aptosConfig,
  );
  return data;
}

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/general}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * general namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptos_config";
import { getAptosFullNode, postAptosFullNode, postAptosIndexer } from "../client";
import { Block, GraphqlQuery, LedgerInfo, LedgerVersion, MoveValue, TableItemRequest, ViewRequest } from "../types";

export async function getLedgerInfo(args: { aptosConfig: AptosConfig }): Promise<LedgerInfo> {
  const { aptosConfig } = args;
  const { data } = await getAptosFullNode<{}, LedgerInfo>({
    aptosConfig,
    originMethod: "getLedgerInfo",
    path: "",
  });
  return data;
}

export async function getBlockByVersion(args: {
  aptosConfig: AptosConfig;
  blockVersion: number;
  options?: { withTransactions?: boolean };
}): Promise<Block> {
  const { aptosConfig, blockVersion, options } = args;
  const { data } = await getAptosFullNode<{}, Block>({
    aptosConfig,
    originMethod: "getBlockByVersion",
    path: `blocks/by_version/${blockVersion}`,
    params: { with_transactions: options?.withTransactions },
  });
  return data;
}

export async function getBlockByHeight(args: {
  aptosConfig: AptosConfig;
  blockHeight: number;
  options?: { withTransactions?: boolean };
}): Promise<Block> {
  const { aptosConfig, blockHeight, options } = args;
  const { data } = await getAptosFullNode<{}, Block>({
    aptosConfig,
    originMethod: "getBlockByHeight",
    path: `blocks/by_height/${blockHeight}`,
    params: { with_transactions: options?.withTransactions },
  });
  return data;
}

export async function getTableItem(args: {
  aptosConfig: AptosConfig;
  handle: string;
  data: TableItemRequest;
  options?: LedgerVersion;
}): Promise<any> {
  const { aptosConfig, handle, data, options } = args;
  const response = await postAptosFullNode<TableItemRequest, any>({
    aptosConfig,
    originMethod: "getTableItem",
    path: `tables/${handle}/item`,
    params: { ledger_version: options?.ledgerVersion },
    body: data,
  });
  return response.data;
}

export async function view(args: {
  aptosConfig: AptosConfig;
  payload: ViewRequest;
  options?: LedgerVersion;
}): Promise<MoveValue[]> {
  const { aptosConfig, payload, options } = args;
  const { data } = await postAptosFullNode<ViewRequest, MoveValue[]>({
    aptosConfig,
    originMethod: "view",
    path: "view",
    params: { ledger_version: options?.ledgerVersion },
    body: payload,
  });
  return data;
}

export async function queryIndexer<T>(args: {
  aptosConfig: AptosConfig;
  query: GraphqlQuery;
  originMethod?: string;
}): Promise<T> {
  const { aptosConfig, query, originMethod } = args;
  const { data } = await postAptosIndexer<GraphqlQuery, T>({
    aptosConfig,
    originMethod: originMethod ?? "queryIndexer",
    path: "",
    body: query,
    overrides: { WITH_CREDENTIALS: false },
  });
  return data;
}

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/account}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * account namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptos_config";
import {
  AccountData,
  LedgerVersion,
  MoveModuleBytecode,
  MoveResource,
  MoveResourceType,
  PaginationArgs,
  TransactionResponse,
  HexInput,
  GetAccountTokensCountQueryResponse,
  GetAccountTokensCountQueryResult,
} from "../types";
import { get, post } from "../client";
import { paginateWithCursor } from "../utils/paginate_with_cursor";
import { AccountAddress } from "../core";
import { AptosApiType } from "../utils/const";
import { GetAccountTokensCount } from "./queries/getAccountTokensAcount";

export async function getInfo(args: { aptosConfig: AptosConfig; accountAddress: HexInput }): Promise<AccountData> {
  const { aptosConfig, accountAddress } = args;
  const { data } = await get<{}, AccountData>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}`,
    originMethod: "getInfo",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getModules(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: PaginationArgs & LedgerVersion;
}): Promise<MoveModuleBytecode[]> {
  const { aptosConfig, accountAddress, options } = args;
  const data = await paginateWithCursor<{}, MoveModuleBytecode[]>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/modules`,
    params: { ledger_version: options?.ledgerVersion, start: options?.start, limit: options?.limit ?? 1000 },
    originMethod: "getModules",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

/**
 * Queries for a move module given account address and module name
 *
 * @param accountAddress Hex-encoded 32 byte Aptos account address
 * @param moduleName The name of the module
 * @param query.ledgerVersion Specifies ledger version of transactions. By default latest version will be used
 * @returns The move module.
 */
export async function getModule(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  moduleName: string;
  options?: LedgerVersion;
}): Promise<MoveModuleBytecode> {
  const { aptosConfig, accountAddress, moduleName, options } = args;
  const { data } = await get<{}, MoveModuleBytecode>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/module/${moduleName}`,
    originMethod: "getModule",
    params: { ledger_version: options?.ledgerVersion },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getTransactions(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: PaginationArgs;
}): Promise<TransactionResponse[]> {
  const { aptosConfig, accountAddress, options } = args;
  const data = await paginateWithCursor<{}, TransactionResponse[]>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/transactions`,
    originMethod: "getTransactions",
    params: { start: options?.start, limit: options?.limit },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getResources(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: PaginationArgs & LedgerVersion;
}): Promise<MoveResource[]> {
  const { aptosConfig, accountAddress, options } = args;
  const data = await paginateWithCursor<{}, MoveResource[]>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/resources`,
    params: { ledger_version: options?.ledgerVersion, start: options?.start, limit: options?.limit ?? 999 },
    originMethod: "getResources",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getResource(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  resourceType: MoveResourceType;
  options?: LedgerVersion;
}): Promise<MoveResource> {
  const { aptosConfig, accountAddress, resourceType, options } = args;
  const { data } = await get<{}, MoveResource>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/resource/${resourceType}`,
    originMethod: "getResource",
    params: { ledger_version: options?.ledgerVersion },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getAccountTokensCount(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
}): Promise<GetAccountTokensCountQueryResult> {
  const { aptosConfig, accountAddress } = args;

  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();

  const whereCondition: any = {
    owner_address: { _eq: address },
    amount: { _gt: "0" },
  };

  const { data } = await post<{}, GetAccountTokensCountQueryResponse>({
    url: aptosConfig.getRequestUrl(AptosApiType.INDEXER),
    body: { query: GetAccountTokensCount, variables: { where_condition: whereCondition } },
    originMethod: "getAccountTokensCount",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data.current_token_ownerships_v2_aggregate.aggregate?.count;
}

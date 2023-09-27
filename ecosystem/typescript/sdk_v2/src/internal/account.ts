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
} from "../types";
import { get } from "../client";
import { paginateWithCursor } from "../utils/paginate_with_cursor";
import { AccountAddress } from "../core";
import { AptosApiType, APTOS_COIN } from "../utils/const";
import { getGasPriceEstimation } from "./transaction";

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

export async function estimateAccountMaxGasAmount(args: { aptosConfig: AptosConfig; accountAddress: HexInput }) {
  const { aptosConfig, accountAddress } = args;
  // Only APT coin is accepted as gas
  const typeTag = `0x1::coin::CoinStore<${APTOS_COIN}>`;
  const [{ gas_estimate: gasUnitPrice }, resources] = await Promise.all([
    getGasPriceEstimation({ aptosConfig }),
    getResources({ aptosConfig, accountAddress }),
  ]);
  const accountResource = resources.find((r) => r.type === typeTag);
  if (!accountResource) {
    throw new Error(`account ${accountAddress} doesnt have a ${typeTag} resource`);
  }
  const balance = BigInt((accountResource.data as any).coin.value);
  return balance / BigInt(gasUnitPrice);
}

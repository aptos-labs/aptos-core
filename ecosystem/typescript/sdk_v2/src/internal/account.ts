import { AptosConfig } from "../api/aptos_config";
import { AnyNumber } from "../bcs";
import { get } from "../client";
import { PaginationArgs } from "../client/types";
import { Gen } from "../types";
import { MaybeHexString, HexString, paginateWithCursor } from "../utils";

export async function getData(aptosConfig: AptosConfig, accountAddress: MaybeHexString): Promise<Gen.AccountData> {
  const { data } = await get<{}, Gen.AccountData>({
    url: aptosConfig.network,
    endpoint: `accounts/${HexString.ensure(accountAddress).hex()}`,
    originMethod: "getData",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getModules(
  aptosConfig: AptosConfig,
  accountAddress: MaybeHexString,
  query?: { ledgerVersion?: AnyNumber },
): Promise<Gen.MoveModuleBytecode[]> {
  const response = await paginateWithCursor<{}, Gen.MoveModuleBytecode[]>({
    url: aptosConfig.network,
    endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/modules`,
    params: { ledger_version: query?.ledgerVersion, limit: 1000 },
    originMethod: "getModules",
    overrides: { ...aptosConfig.clientConfig },
  });
  return response;
}

/**
 * Queries module associated with given account by module name
 *
 * Note: In order to get all account resources, this function may call the API
 * multiple times as it paginates.
 *
 * @param accountAddress Hex-encoded 32 byte Aptos account address
 * @param moduleName The name of the module
 * @param query.ledgerVersion Specifies ledger version of transactions. By default latest version will be used
 * @returns Specified module.
 * Module is represented by MoveModule interface. It contains module `bytecode` and `abi`,
 * which JSON representation of a module
 */
export async function getModule(
  aptosConfig: AptosConfig,
  accountAddress: MaybeHexString,
  moduleName: string,
  query?: { ledgerVersion?: AnyNumber },
): Promise<Gen.MoveModuleBytecode> {
  const { data } = await get<{}, Gen.MoveModuleBytecode>({
    url: aptosConfig.network,
    endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/module/${moduleName}`,
    originMethod: "getModule",
    params: { ledger_version: query?.ledgerVersion },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getTransactions(
  aptosConfig: AptosConfig,
  accountAddress: MaybeHexString,
  query?: PaginationArgs,
): Promise<Gen.Transaction[]> {
  const { data } = await get<{}, Gen.Transaction[]>({
    url: aptosConfig.network,
    endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/transactions`,
    originMethod: "getTransactions",
    params: { start: query?.start, limit: query?.limit },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getResources(
  aptosConfig: AptosConfig,
  accountAddress: MaybeHexString,
  query?: { ledgerVersion?: AnyNumber },
): Promise<Gen.MoveResource[]> {
  const out = await paginateWithCursor<{}, Gen.MoveResource[]>({
    url: aptosConfig.network,
    endpoint: `accounts/${accountAddress}/resources`,
    params: { ledger_version: query?.ledgerVersion, limit: 9999 },
    originMethod: "getResources",
    overrides: { ...aptosConfig.clientConfig },
  });
  return out;
}

export async function getResource(
  aptosConfig: AptosConfig,
  accountAddress: MaybeHexString,
  resourceType: Gen.MoveStructTag,
  query?: { ledgerVersion?: AnyNumber },
): Promise<Gen.MoveResource> {
  const { data } = await get<{}, Gen.MoveResource>({
    url: aptosConfig.network,
    endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/resource/${resourceType}`,
    originMethod: "getResource",
    params: { ledger_version: query?.ledgerVersion },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

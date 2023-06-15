import { AptosConfig } from "../api/aptos_config";
import { get, post } from "../client";
import { Gen } from "../types";

// TODO memoize
export async function getChainId(aptosConfig: AptosConfig): Promise<number> {
  const { data } = await get<{}, Gen.IndexResponse>({
    url: aptosConfig.network,
    originMethod: "getChainId",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data.chain_id;
}

export async function getLedgerInfo(aptosConfig: AptosConfig): Promise<Gen.IndexResponse> {
  const { data } = await get<{}, Gen.IndexResponse>({
    url: aptosConfig.network,
    originMethod: "getLedgerInfo",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function view(
  aptosConfig: AptosConfig,
  payload: Gen.ViewRequest,
  ledger_version?: string,
): Promise<Gen.MoveValue[]> {
  const { data } = await post<Gen.ViewRequest, Gen.MoveValue[]>({
    url: aptosConfig.network,
    body: payload,
    endpoint: "view",
    originMethod: "view",
    params: { ledger_version },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getBlockByHeight(
  aptosConfig: AptosConfig,
  blockHeight: number,
  withTransactions?: boolean,
): Promise<Gen.Block> {
  const { data } = await get<{}, Gen.Block>({
    url: aptosConfig.network,
    endpoint: `blocks/by_height/${blockHeight}`,
    originMethod: "getBlockByHeight",
    params: { with_transactions: withTransactions },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

export async function getBlockByVersion(
  aptosConfig: AptosConfig,
  version: number,
  withTransactions?: boolean,
): Promise<Gen.Block> {
  const { data } = await get<{}, Gen.Block>({
    url: aptosConfig.network,
    endpoint: `blocks/by_version/${version}`,
    originMethod: "getBlockByVersion",
    params: { with_transactions: withTransactions },
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

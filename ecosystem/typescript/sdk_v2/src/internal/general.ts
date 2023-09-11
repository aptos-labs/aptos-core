import { AptosConfig } from "../api";
import { get } from "../client";
import { LedgerInfo } from "../types";
import { AptosApiType } from "../utils/const";

export async function getLedgerInfo(args: { aptosConfig: AptosConfig }): Promise<LedgerInfo> {
  const { aptosConfig } = args;
  const { data } = await get<{}, LedgerInfo>({
    url: aptosConfig.getRequestUrl(AptosApiType.FULLNODE),
    endpoint: `/`,
    originMethod: "getLedgerInfo",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

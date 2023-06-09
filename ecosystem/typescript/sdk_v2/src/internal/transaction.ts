import { AptosConfig } from "../api/aptos_config";
import { get } from "../client";
import { Gen } from "../types";

export async function estimateGasPrice(aptosConfig: AptosConfig): Promise<Gen.GasEstimation> {
  const { data } = await get<{}, Gen.GasEstimation>({
    url: aptosConfig.network,
    endpoint: "estimate_gas_price",
    originMethod: "estimateGasPrice",
    overrides: { ...aptosConfig.clientConfig },
  });
  return data;
}

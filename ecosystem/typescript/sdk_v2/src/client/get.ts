import { AptosResponse } from "./types";
import { aptosRequest } from "./core";
import { AptosRequest } from "../types";
import { AptosConfig } from "../api/aptos_config";

export type GetRequestOptions = Omit<AptosRequest, "body" | "method">;

/**
 * Main function to do a Get request
 *
 * @param options GetRequestOptions
 * @param aptosConfig The config information for the SDK client instance
 * @returns
 */
export async function get<Req, Res>(
  options: GetRequestOptions,
  aptosConfig: AptosConfig,
): Promise<AptosResponse<Req, Res>> {
  const response: AptosResponse<Req, Res> = await aptosRequest<Req, Res>(
    { ...options, method: "GET", overrides: { ...aptosConfig.clientConfig, ...options.overrides } },
    aptosConfig,
  );
  return response;
}

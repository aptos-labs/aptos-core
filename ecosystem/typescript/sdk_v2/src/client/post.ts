import { AptosConfig } from "../api/aptos_config";
import { AptosRequest } from "../types";
import { aptosRequest } from "./core";
import { AptosResponse } from "./types";

export type PostRequestOptions = Omit<AptosRequest, "method">;

/**
 * Main function to do a Post request
 *
 * @param options PostRequestOptions
 * @param aptosConfig The config information for the SDK client instance
 * @returns
 */
export async function post<Req, Res>(
  options: PostRequestOptions,
  aptosConfig: AptosConfig,
): Promise<AptosResponse<Req, Res>> {
  const response: AptosResponse<Req, Res> = await aptosRequest<Req, Res>(
    { ...options, method: "POST", overrides: { ...aptosConfig.clientConfig, ...options.overrides } },
    aptosConfig,
  );
  return response;
}

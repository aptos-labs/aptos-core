import aptosClient from "@aptos-labs/aptos-client";
import { AptosApiError, AptosResponse } from "./types";
import { VERSION } from "../version";
import { ClientConfig, AptosRequest } from "../types";
import { AptosConfig } from "../api/aptos_config";

/**
 * Meaningful errors map
 */
const errors: Record<number, string> = {
  400: "Bad Request",
  401: "Unauthorized",
  403: "Forbidden",
  404: "Not Found",
  429: "Too Many Requests",
  500: "Internal Server Error",
  502: "Bad Gateway",
  503: "Service Unavailable",
};

/**
 * Given a url and method, sends the request with axios and
 * returns the response.
 */
async function request<Req, Res>(
  url: string,
  method: "GET" | "POST",
  body?: Req,
  contentType?: string,
  params?: any,
  overrides?: ClientConfig,
): Promise<any> {
  const headers: Record<string, string | string[] | undefined> = {
    ...overrides?.HEADERS,
    "x-aptos-client": `aptos-ts-sdk/${VERSION}`,
    "content-type": contentType ?? "application/json",
  };

  if (overrides?.TOKEN) {
    headers.Authorization = `Bearer ${overrides?.TOKEN}`;
  }

  /**
   * make a call using the @aptos-labs/aptos-client package
   * {@link https://www.npmjs.com/package/@aptos-labs/aptos-client}
   */
  const response = await aptosClient<Res>({ url, method, body, params, headers, overrides });
  return response;
}

/**
 * The main function to use when doing an API request.
 *
 * @param options AptosRequest
 * @param aptosConfig The config information for the SDK client instance
 * @returns the response or AptosApiError
 */
export async function aptosRequest<Req, Res>(
  options: AptosRequest,
  aptosConfig: AptosConfig,
): Promise<AptosResponse<Req, Res>> {
  const { url, path, method, body, contentType, params, overrides } = options;
  const fullUrl = `${url}/${path ?? ""}`;
  const response = await request<Req, Res>(fullUrl, method, body, contentType, params, overrides);
  const result: AptosResponse<Req, Res> = {
    status: response.status,
    statusText: response.statusText!,
    data: response.data,
    headers: response.headers,
    config: response.config,
    url: fullUrl,
  };

  // to support both fullnode and indexer responses,
  // check if it is an indexer query, and adjust response.data
  if (aptosConfig.isIndexerRequest(url)) {
    // errors from indexer
    if ((result.data as any).errors) {
      throw new AptosApiError(options, result, response.data.errors[0].message ?? "Generic Error");
    }
    result.data = (result.data as any).data as Res;
  }

  if (result.status >= 200 && result.status < 300) {
    return result;
  }
  const errorMessage = errors[result.status];
  throw new AptosApiError(options, result, errorMessage ?? "Generic Error");
}

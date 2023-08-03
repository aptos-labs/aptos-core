import aptosClient from "@aptos-labs/aptos-client";
import { AptosApiError, AptosRequest, AptosResponse, ClientConfig } from "./types";
import { VERSION } from "../version";

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
 * @returns the response or AptosApiError
 */
export async function aptosRequest<Req, Res>(options: AptosRequest): Promise<AptosResponse<Req, Res>> {
  const { url, endpoint, method, body, contentType, params, overrides } = options;
  const fullEndpoint = `${url}/${endpoint ?? ""}`;
  const response = await request<Req, Res>(fullEndpoint, method, body, contentType, params, overrides);

  const result: AptosResponse<Req, Res> = {
    status: response.status,
    statusText: response.statusText!,
    data: response.data,
    headers: response.headers,
    config: response.config,
    url: fullEndpoint,
  };

  if (result.status >= 200 && result.status < 300) {
    return result;
  }
  const errorMessage = errors[result.status];
  throw new AptosApiError(options, result, errorMessage ?? "Generic Error");
}

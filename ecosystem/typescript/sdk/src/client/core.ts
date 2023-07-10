import axios, { AxiosResponse, AxiosRequestConfig, AxiosError } from "axios";
import { AnyNumber } from "../bcs/types";
import { VERSION } from "../version";
import { AptosApiError, AptosRequest, AptosResponse, ClientConfig } from "./types";
import "./cookieJar";

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
async function axiosRequest<Request, Response>(
  url: string,
  method: "GET" | "POST",
  body?: Request,
  contentType?: string,
  params?: Record<string, string | AnyNumber | boolean | undefined>,
  overrides?: ClientConfig,
): Promise<AxiosResponse<Response>> {
  const headers: Record<string, string | number | boolean> = {
    ...overrides?.HEADERS,
    "x-aptos-client": `aptos-ts-sdk/${VERSION}`,
    "content-type": contentType ?? "application/json",
  };

  if (overrides?.TOKEN) {
    headers.Authorization = `Bearer ${overrides?.TOKEN}`;
  }

  const requestConfig: AxiosRequestConfig = {
    headers,
    method,
    url,
    params,
    data: body,
    withCredentials: overrides?.WITH_CREDENTIALS ?? true,
  };

  try {
    return await axios(requestConfig);
  } catch (error) {
    const axiosError = error as AxiosError<Response>;
    if (axiosError.response) {
      return axiosError.response;
    }
    throw error;
  }
}

/**
 * The main function to use when doing an API request.
 * Wraps axios error response with AptosApiError
 *
 * @param options AptosRequest
 * @returns the response or AptosApiError
 */
export async function aptosRequest<Req, Res>(options: AptosRequest): Promise<AptosResponse<Req, Res>> {
  const { url, endpoint, method, body, contentType, params, overrides } = options;
  const fullEndpoint = `${url}/${endpoint ?? ""}`;
  const response = await axiosRequest<Req, Res>(fullEndpoint, method, body, contentType, params, overrides);

  const result: AptosResponse<Req, Res> = {
    status: response.status,
    statusText: response.statusText,
    data: response.data,
    headers: response.headers,
    config: response.config,
    url: fullEndpoint,
  };

  if (response.status >= 200 && response.status < 300) {
    return result;
  }
  const errorMessage = errors[response.status];
  throw new AptosApiError(options, result, errorMessage ?? "Generic Error");
}

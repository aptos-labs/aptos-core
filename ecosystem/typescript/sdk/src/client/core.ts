import axios, { AxiosResponse, AxiosRequestConfig, AxiosError } from "axios";
import { AnyNumber } from "../bcs/types";
import { VERSION } from "../version";
import "./cookieJar";

/**
 * A configuration object we can pass with the request to the server.
 * HEADERS - extra headers we want to send with the request
 * TOKEN - an auth token to send with the request
 * WITH_CREDENTIALS - whether to carry cookies. By default, it is set to true and cookies will be sent
 */
export type ClientConfig = {
  TOKEN?: string;
  HEADERS?: Record<string, string | number | boolean>;
  WITH_CREDENTIALS?: boolean;
};

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
 * The request type returned from an API error
 */
type AptosApiErrorRequest = {
  url: string;
  method: string;
  originMethod: string | undefined;
};

/**
 * The response type returned from an API error
 */
type AptosApiErrorResponse = {
  status: number;
  statusText: string;
  body: any;
  url: string;
};

/**
 * The type returned from an API error
 */
export class AptosApiError extends Error {
  readonly url: string;

  readonly status: number;

  readonly statusText: string;

  readonly body: any;

  readonly request: AptosApiErrorRequest;

  constructor(request: AptosApiErrorRequest, response: AptosApiErrorResponse, message: string) {
    super(message);

    this.name = "AptosApiError";
    this.url = response.url;
    this.status = response.status;
    this.statusText = response.statusText;
    this.body = response.body;
    this.request = request;
  }
}

/**
 * Given a url and method, sends the request with axios and
 * returns the response.
 */
async function axiosRequest<Request, Response>(
  url: string,
  method: "GET" | "POST",
  body?: Request,
  params?: Record<string, string | AnyNumber | boolean | undefined>,
  overrides?: ClientConfig,
): Promise<AxiosResponse<Response>> {
  const headers: Record<string, string | number | boolean> = {
    ...overrides?.HEADERS,
    "x-aptos-client": `aptos-ts-sdk/${VERSION}`,
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
    // Do not carry cookies when `WITH_CREDENTIALS` is explicitly set to `false`. By default, cookies will be sent
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

export type AptosRequest<Req> = {
  url: string;
  method: "GET" | "POST";
  endpoint?: string;
  body?: Req;
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  originMethod?: string;
  overrides?: ClientConfig;
};

/**
 * The main function to use when doing an API request.
 * Wraps axios error response with AptosApiError
 *
 * @param options AptosRequest
 * @returns the response or AptosApiError
 */
export async function aptosRequest<Req, Res>(
  options: AptosRequest<Req>,
): Promise<AxiosResponse<Res, any> | AptosApiError> {
  const { url, endpoint, method, body, params, overrides, originMethod } = options;
  const fullEndpoint = `${url}/${endpoint ?? ""}`;
  const response = await axiosRequest<Req, Res>(fullEndpoint, method, body, params, overrides);
  if (response.status >= 200 && response.status < 300) {
    return response;
  }
  const errorMessage = errors[response.status];
  throw new AptosApiError(
    { url: fullEndpoint, method, originMethod },
    { status: response.status, statusText: response.statusText, body: response.data, url: fullEndpoint },
    errorMessage ?? "Generic Error",
  );
}

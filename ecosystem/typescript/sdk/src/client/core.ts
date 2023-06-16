import axios, { AxiosResponse, AxiosRequestConfig, AxiosError } from "axios";
import { AnyNumber } from "../bcs/types";
import { VERSION } from "../version";

/**
 * Client config to override or pass more data to the request
 */
export type ClientConfig = {
  token?: string;
  headers?: Record<string, string | number | boolean>;
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
 * The request type returned from an api error
 */
type AptosApiErrorRequest = {
  url: string;
  method: string;
  originMethod: string | undefined;
};

/**
 * The response type returned from an api error
 */
type AptosApiErrorResponse = {
  status: number;
  statusText: string;
  body: any;
  url: string;
};

/**
 * The type returned from an api error
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
    ...overrides?.headers,
    "x-aptos-client": `aptos-ts-sdk/${VERSION}`,
  };

  if (overrides?.token) {
    headers.Authorization = `Bearer ${overrides?.token}`;
  }

  const requestConfig: AxiosRequestConfig = {
    headers,
    method,
    url,
    params,
    data: body,
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
 * The main function to use when doing an api request.
 * Wraps axios error response with AptosApiError
 *
 * @param url the base url for the request
 * @param method the request method - GET or POST
 * @returns the response or AptosApiError
 */
export async function aptosRequest<Req, Res>(options: AptosRequest<Req>): Promise<AxiosResponse<Res, any>> {
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

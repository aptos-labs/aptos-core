import axios, { Axios, AxiosRequestConfig, AxiosResponse } from "axios";
import { AptosConfig } from "../aptos_config";
import { VERSION } from "../version";

class AptosApiError {
  constructor(
    public readonly status: number,
    public readonly message: string,
    public readonly originMethod: string | undefined,
    public readonly destinationURL: string,
  ) {}
}

/**
 * Given an endpoint, method, and params, sends the request with axios and
 * returns the response.
 */
async function request<Request, Response>(
  url: string,
  method: "GET" | "POST",
  formData?: FormData,
  params?: Request,
  body?: any,
  overrides?: AxiosRequestConfig,
): Promise<AxiosResponse<Response>> {
  const requestConfig: AxiosRequestConfig = {
    headers: {
      ...overrides?.headers,
      "x-aptos-client": `aptos-ts-sdk/${VERSION}`,
    },
    method,
    url,
    params,
    data: body ?? formData,
  };

  return await axios.request(requestConfig);
}

/**
 * Wrap axios request with AptosRequestError
 * @param params
 * @param originMethod
 * @param destinationURL
 * @param overrides
 * @returns
 */
export async function aptosRequest<Req, Res>(
  config: AptosConfig,
  url: string,
  method: "GET" | "POST",
  body?: any,
  params?: Req,
  originMethod?: string,
  overrides?: AxiosRequestConfig,
): Promise<Res> {
  // retry logic goes here
  const fullEndpoint = `${config.network}/${url}`;
  try {
    const response = await request<Req, Res>(fullEndpoint, method, body, params, overrides);
    if (response.status === 200) {
      return response.data;
    } else {
      const error = new AptosApiError(response.status, `${response.data}`, originMethod, fullEndpoint);
      // log full error data
      // TODO - use LogService
      console.error("error", error);
      console.trace();
      // Promise.reject returns only the `message` property in `AptosApiError` class
      return Promise.reject(error);
    }
  } catch (err: any) {
    if (!axios.isAxiosError(err) || err.response === undefined) {
      throw err;
    }
    const error = new AptosApiError(err.response.status, JSON.stringify(err.response.data), originMethod, fullEndpoint);
    // log full error data
    // TODO - use LogService
    console.debug("error", error);
    console.trace();
    // Promise.reject returns only the `message` property in `AptosApiError` class

    return Promise.reject(error);
  }
}

export async function get<Req, Res>(
  config: AptosConfig,
  url: string,
  params?: Req,
  originMethod?: string,
  overrides?: AxiosRequestConfig,
): Promise<Res> {
  return aptosRequest(config, url, "GET", null, params, originMethod, overrides);
}

export async function post<Req, Res>(
  config: AptosConfig,
  url: string,
  body?: Req,
  originMethod?: string,
  overrides?: AxiosRequestConfig,
): Promise<Res> {
  return aptosRequest(config, url, "POST", body, null, originMethod, overrides);
}

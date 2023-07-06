import { AxiosResponse } from "axios";
import { AnyNumber } from "../bcs";

/**
 * A configuration object we can pass with the request to the server.
 *
 * @param TOKEN - an auth token to send with the request
 * @param HEADERS - extra headers we want to send with the request
 * @param WITH_CREDENTIALS - whether to carry cookies. By default, it is set to true and cookies will be sent
 */
export type ClientConfig = {
  TOKEN?: string;
  HEADERS?: Record<string, string | number | boolean>;
  WITH_CREDENTIALS?: boolean;
};

/**
 * The API request type
 *
 * @param url - the url to make the request to, i.e https://fullnode.aptoslabs.devnet.com/v1
 * @param method - the request method "GET" | "POST"
 * @param endpoint (optional) - the endpoint to make the request to, i.e transactions
 * @param body (optional) - the body of the request
 * @param contentType (optional) - the content type to set the `content-type` header to,
 * by default is set to `application/json`
 * @param params (optional) - query params to add to the request
 * @param originMethod (optional) - the local method the request came from
 * @param overrides (optional) - a `ClientConfig` object type to override request data
 */
export type AptosRequest = {
  url: string;
  method: "GET" | "POST";
  endpoint?: string;
  body?: any;
  contentType?: string;
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  originMethod?: string;
  overrides?: ClientConfig;
};

/**
 * The response type returned from an API error
 */
export interface AptosResponse<Req, Res> extends AxiosResponse<Res, ClientConfig> {
  status: number;
  statusText: string;
  data: Res;
  url: string;
  request?: Req;
}

/**
 * The type returned from an API error
 */
export class AptosApiError extends Error {
  readonly url: string;

  readonly status: number;

  readonly statusText: string;

  readonly data: any;

  readonly request: AptosRequest;

  constructor(request: AptosRequest, response: AptosResponse<any, any>, message: string) {
    super(message);

    this.name = "AptosApiError";
    this.url = response.url;
    this.status = response.status;
    this.statusText = response.statusText;
    this.data = response.data;
    this.request = request;
  }
}

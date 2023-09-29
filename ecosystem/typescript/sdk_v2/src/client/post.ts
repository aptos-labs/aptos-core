import { AptosConfig } from "../api/aptos_config";
import { AnyNumber, ClientConfig } from "../types";
import { aptosRequest } from "./core";
import { AptosResponse } from "./types";
import { AptosApiType } from "../utils/const";

export type PostRequestOptions = {
  /**
   * The config for the API client
   */
  aptosConfig: AptosConfig;
  /**
   * The type of API endpoint to call e.g. fullnode, indexer, etc
   */
  type: AptosApiType;
  /**
   * The name of the API method
   */
  originMethod: string;
  /**
   * The URL path to the API method
   */
  path: string;
  /**
   * The content type of the request
   */
  contentType?: string;
  /**
   * The query parameters for the request
   */
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  /**
   * The body of the request, should match teh content type of the request
   */
  body?: any;
  /**
   * Specific client overrides for this request to override aptosConfig
   */
  overrides?: ClientConfig;
};

export type PostAptosFullNodeRequestOptions = Omit<PostRequestOptions, "type">;

/**
 * Main function to do a Post request
 *
 * @param options PostRequestOptions
 * @returns
 */
export async function post<Req, Res>(options: PostRequestOptions): Promise<AptosResponse<Req, Res>> {
  const { type, originMethod, path, body, contentType, params, aptosConfig, overrides } = options;
  const url = aptosConfig.getRequestUrl(type);

  const response: AptosResponse<Req, Res> = await aptosRequest<Req, Res>(
    {
      url,
      method: "POST",
      originMethod,
      path,
      body,
      contentType,
      params,
      overrides: {
        ...aptosConfig,
        ...overrides,
      },
    },
    aptosConfig,
  );
  return response;
}

export async function postAptosFullNode<Req, Res>(
  options: PostAptosFullNodeRequestOptions,
): Promise<AptosResponse<Req, Res>> {
  return post<Req, Res>({ ...options, type: AptosApiType.FULLNODE });
}

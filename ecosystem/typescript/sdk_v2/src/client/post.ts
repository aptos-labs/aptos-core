import { AptosConfig } from "../api/aptos_config";
import {AnyNumber, ClientConfig} from "../types";
import { aptosRequest } from "./core";
import { AptosResponse } from "./types";
import {AptosApiType} from "../utils/const";

export type PostRequestOptions = {
  aptosConfig: AptosConfig;
  type: AptosApiType,
  name: string;
  path: string;
  contentType?: string;
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  body?: any;
  overrides?: ClientConfig;
};

export type PostFullNodeRequestOptions =  Omit<PostRequestOptions, "type">;

/**
 * Main function to do a Post request
 *
 * @param options PostRequestOptions
 * @param aptosConfig The config information for the SDK client instance
 * @returns
 */
export async function post<Req, Res>(
  options: PostRequestOptions,
): Promise<AptosResponse<Req, Res>> {
  const url = options.aptosConfig.getRequestUrl(options.type);

  const response: AptosResponse<Req, Res> = await aptosRequest<Req, Res>(
      {
        url,
        method: "POST",
        name: options.name,
        path: options.path,
        contentType: options.contentType,
        params: options.params,
        overrides: {
          ...options.aptosConfig, ...options.overrides
        }
      },
      options.aptosConfig
  );
  return response;
}

export async function postFullnode<Req, Res>(
    options: PostFullNodeRequestOptions,
): Promise<AptosResponse<Req, Res>> {
  return post<Req, Res>({...options, type: AptosApiType.FULLNODE});
}

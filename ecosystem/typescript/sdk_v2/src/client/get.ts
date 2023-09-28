import {AptosResponse} from "./types";
import {aptosRequest} from "./core";
import {AnyNumber, ClientConfig} from "../types";
import {AptosConfig} from "../api/aptos_config";
import {AptosApiType} from "../utils/const";

export type GetRequestOptions = {
  aptosConfig: AptosConfig;
  type: AptosApiType,
  name: string;
  path: string;
  contentType?: string;
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  overrides?: ClientConfig;
};

export type GetFullNodeRequestOptions =  Omit<GetRequestOptions, "type">;

/**
 * Main function to do a Get request
 *
 * @param options GetRequestOptions
 * @param aptosConfig The config information for the SDK client instance
 * @returns
 */
export async function get<Req, Res>(
  options: GetRequestOptions
): Promise<AptosResponse<Req, Res>> {
  const url = options.aptosConfig.getRequestUrl(options.type);

  const response: AptosResponse<Req, Res> = await aptosRequest<Req, Res>(
      {
        url,
        method: "GET",
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

export async function getFullNode<Req, Res>(
    options: GetFullNodeRequestOptions
): Promise<AptosResponse<Req, Res>> {
    return get<Req, Res>({...options, type: AptosApiType.FULLNODE});
}

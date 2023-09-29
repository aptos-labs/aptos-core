import { AptosResponse } from "./types";
import { aptosRequest } from "./core";
import { AnyNumber, ClientConfig } from "../types";
import { AptosConfig } from "../api/aptos_config";
import { AptosApiType } from "../utils/const";

export type GetRequestOptions = {
  aptosConfig: AptosConfig;
  type: AptosApiType;
  name: string;
  path: string;
  contentType?: string;
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  overrides?: ClientConfig;
};

export type GetFullNodeRequestOptions = Omit<GetRequestOptions, "type">;

/**
 * Main function to do a Get request
 *
 * @param options GetRequestOptions
 * @param aptosConfig The config information for the SDK client instance
 * @returns
 */
export async function get<Req, Res>(options: GetRequestOptions): Promise<AptosResponse<Req, Res>> {
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
        ...options.aptosConfig,
        ...options.overrides,
      },
    },
    options.aptosConfig,
  );
  return response;
}

export async function getFullNode<Req, Res>(options: GetFullNodeRequestOptions): Promise<AptosResponse<Req, Res>> {
  return get<Req, Res>({ ...options, type: AptosApiType.FULLNODE });
}

/// This function is a helper for paginating using a function wrapping an API
export async function paginateWithCursor<Req extends Record<string, any>, Res extends any[]>(
  options: GetFullNodeRequestOptions,
): Promise<Res> {
  const out = [];
  let cursor: string | undefined;
  const requestParams = options.params as Req & { start?: string };
  // eslint-disable-next-line no-constant-condition
  while (true) {
    requestParams.start = cursor;
    // eslint-disable-next-line no-await-in-loop
    const response = await getFullNode<Req, Res>({
      aptosConfig: options.aptosConfig,
      name: options.name,
      path: options.path,
      params: requestParams,
      overrides: options.overrides,
    });
    /**
     * the cursor is a "state key" from the API prespective. Client
     * should not need to "care" what it represents but just use it
     * to query the next chunck of data.
     */
    cursor = response.headers["x-aptos-cursor"];
    // Now that we have the cursor (if any), we remove the headers before
    // adding these to the output of this function.
    delete (response as any).headers;
    out.push(...response.data);
    if (cursor === null || cursor === undefined) {
      break;
    }
  }
  return out as any;
}

import { AptosConfig } from "../api/aptos_config";
import {get, getFullNode, GetFullNodeRequestOptions} from "../client";
import { AptosRequest } from "../types";

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
    const response = await getFullNode<Req, Res>(
      {
        aptosConfig: options.aptosConfig,
        name: options.name,
        path: options.path,
        params: requestParams,
        overrides: options.overrides,
      },
    );
    // eslint-disable-next-line no-underscore-dangle
    /**
     * the cursor is a "state key" from the API prespective. Client
     * should not need to "care" what it represents but just use it
     * to query the next chunck of data.
     */
    cursor = response.headers["x-aptos-cursor"];
    // Now that we have the cursor (if any), we remove the headers before
    // adding these to the output of this function.
    // eslint-disable-next-line no-underscore-dangle
    delete (response as any).headers;
    out.push(...response.data);
    if (cursor === null || cursor === undefined) {
      break;
    }
  }
  return out as any;
}

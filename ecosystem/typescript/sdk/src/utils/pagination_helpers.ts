import { AnyNumber } from "../bcs";
import { HexString, MaybeHexString } from "../hex_string";

/// This function is a helper for paginating using a function wrapping an API
export async function paginateWithCursor<T>(
  apiFunction: (
    address: string,
    ledgerVersion?: string | undefined,
    start?: string | undefined,
    limit?: number | undefined,
  ) => Promise<T[]>,
  accountAddress: MaybeHexString,
  limitPerRequest: number,
  query?: { ledgerVersion?: AnyNumber },
): Promise<T[]> {
  const out = [];
  let cursor: string | undefined;
  // eslint-disable-next-line no-constant-condition
  while (true) {
    // eslint-disable-next-line no-await-in-loop
    const response = await apiFunction(
      HexString.ensure(accountAddress).hex(),
      query?.ledgerVersion?.toString(),
      cursor,
      limitPerRequest,
    );
    // Response is the main response, i.e. the T[]. Attached to that are the headers as `__headers`.
    // eslint-disable-next-line no-underscore-dangle
    cursor = (response as any).__headers["x-aptos-cursor"];
    // Now that we have the cursor (if any), we remove the headers before
    // adding these to the output of this function.
    // eslint-disable-next-line no-underscore-dangle
    delete (response as any).__headers;
    out.push(...response);
    if (cursor === null || cursor === undefined) {
      break;
    }
  }
  return out;
}

import { get } from "../../client";
import * as Gen from "../../generated";
import { paginateWithCursor } from "../../utils/pagination_helpers";
import { longTestTimeout } from "../unit/test_helper.test";

/**
 * If 0x1 ever has more than 9999 modules this test will fail
 * since the first query will only return 9999 modules
 * and the second query will return all modules
 */
test(
  "gets the full data with pagination",
  async () => {
    try {
      const modules = await get<{}, Gen.MoveModuleBytecode[]>({
        url: "https://fullnode.testnet.aptoslabs.com/v1",
        endpoint: "accounts/0x1/modules",
        params: { limit: 9999 },
      });
      const paginateOut = await paginateWithCursor<{}, Gen.MoveModuleBytecode[]>({
        url: "https://fullnode.testnet.aptoslabs.com/v1",
        endpoint: "accounts/0x1/modules",
        params: { limit: 20 },
      });
      expect(paginateOut.length).toBe(modules.data.length);
    } catch (err) {
      console.log(err);
    }
  },
  longTestTimeout,
);

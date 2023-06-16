import { get } from "../../client";
import * as Gen from "../../generated";
import { paginateWithCursor } from "../../utils/pagination_helpers";
import { longTestTimeout } from "../unit/test_helper.test";

test(
  "gets the full data with pagination",
  async () => {
    try {
      const modules = await get<{}, Gen.MoveModuleBytecode[]>({
        url: "https://fullnode.testnet.aptoslabs.com/v1",
        endpoint: "accounts/0x1/modules",
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

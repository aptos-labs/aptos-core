import { AptosClient } from "../../providers";
import { longTestTimeout, NODE_URL } from "./test_helper.test";
import * as aptosClient from "../../client";
import { MoveFunctionVisibility } from "../../generated";

jest.mock("../../client", () => {
  return {
    __esModule: true,
    ...jest.requireActual("../../client"),
  };
});

describe("Account Modules", () => {
  test(
    "memoize account modules",
    async () => {
      const client = new AptosClient(NODE_URL);
      const moduleAddress = "0x1";
      const accountModulesResponse = [
        {
          bytecode: "0xa11ceb0b060000000e",
          abi: {
            address: "0x1",
            name: "some_modules",
            friends: [],
            exposed_functions: [
              {
                name: "SomeName",
                visibility: MoveFunctionVisibility.PUBLIC,
                is_entry: true,
                is_view: false,
                generic_type_params: [{ constraints: [] }],
                params: ["&signer", "0x1::string::String"],
                return: [],
              },
            ],
            structs: [],
          },
        },
      ];

      const getSpy = jest.spyOn(aptosClient, "get").mockResolvedValue({
        status: 200,
        statusText: "OK",
        data: accountModulesResponse,
        url: "/fullnode/v1/accounts/0x1/modules",
        headers: {},
      });

      const accountModules = await client.getAccountModules(moduleAddress);
      expect(accountModules).toEqual(accountModulesResponse);

      const accountModulesWithSameAddress = await client.getAccountModules(moduleAddress);
      expect(accountModulesWithSameAddress).toEqual(accountModulesResponse);

      // make sure it does not make a request again
      expect(getSpy).toBeCalledTimes(1);
      getSpy.mockRestore();
    },
    longTestTimeout,
  );
});

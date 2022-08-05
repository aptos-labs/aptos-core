import { fromMoveStructTagParam, toMoveStructTagParam } from "./util";

export const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com/v1";
export const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

test("noop", () => {
  // All TS files are compiled by default into the npm package
  // Adding this empty test allows us to:
  // 1. Guarantee that this test library won't get compiled
  // 2. Prevent jest from exploding when it finds a file with no tests in it
});

test("toMoveStructTagParam", () => {
  const moveStructTag1 = {
    address: "0x1",
    module: "coin",
    name: "CoinStore",
    generic_type_params: ["0x1::aptos_coin::AptosCoin", "0x3::token::Whatever"],
  };
  let actual1 = toMoveStructTagParam(moveStructTag1);
  expect(actual1).toBe("0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin,0x3::token::Whatever>");

  const moveStructTag2 = {
    address: "0x1",
    module: "coin",
    name: "CoinStore",
    generic_type_params: [] as string[],
  };
  let actual2 = toMoveStructTagParam(moveStructTag2);
  expect(actual2).toBe("0x1::coin::CoinStore");
});

test("fromMoveStructTagParam", () => {
  const moveStructTagParam1 = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin,0x3::token::Whatever>";
  let actual1 = fromMoveStructTagParam(moveStructTagParam1);
  expect(actual1).toStrictEqual({
    address: "0x1",
    module: "coin",
    name: "CoinStore",
    generic_type_params: ["0x1::aptos_coin::AptosCoin", "0x3::token::Whatever"],
  });

  const moveStructTagParam2 = "0x1::coin::CoinStore";
  let actual2 = fromMoveStructTagParam(moveStructTagParam2);
  expect(actual2).toStrictEqual({
    address: "0x1",
    module: "coin",
    name: "CoinStore",
    generic_type_params: [],
  });
});

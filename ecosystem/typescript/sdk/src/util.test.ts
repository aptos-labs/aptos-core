// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from "./aptos_client.js";

export const NODE_URL = process.env.APTOS_NODE_URL;
export const FAUCET_URL = process.env.APTOS_FAUCET_URL;

test("noop", () => {
  // All TS files are compiled by default into the npm package
  // Adding this empty test allows us to:
  // 1. Guarantee that this test library won't get compiled
  // 2. Prevent jest from exploding when it finds a file with no tests in it
});

test("test fixNodeUrl", () => {
  expect(new AptosClient("https://test.com").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/v1").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/v1/").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com", {}, true).client.request.config.BASE).toBe("https://test.com");
});

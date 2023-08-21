// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from "../../providers/aptos_client";

test("test fixNodeUrl", () => {
  expect(new AptosClient("https://test.com").nodeUrl).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/").nodeUrl).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/v1").nodeUrl).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/v1/").nodeUrl).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com", {}, true).nodeUrl).toBe("https://test.com");
});

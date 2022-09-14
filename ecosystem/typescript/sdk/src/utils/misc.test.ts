// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from "../aptos_client";

test("test fixNodeUrl", () => {
  expect(new AptosClient("https://test.com").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/v1").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com/v1/").client.request.config.BASE).toBe("https://test.com/v1");
  expect(new AptosClient("https://test.com", {}, true).client.request.config.BASE).toBe("https://test.com");
});

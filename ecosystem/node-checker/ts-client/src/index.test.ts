// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

import { ApiError, NodeCheckerClient } from "./index";

test("getConfigurationKeys", async () => {
  const client = new NodeCheckerClient({
    BASE: "http://127.0.0.1:20121",
  });
  const keys = await client.default.getConfigurations();
  expect(keys.length).toBeGreaterThan(0);
});

test("checkNode", async () => {
  const client = new NodeCheckerClient({
    BASE: "http://127.0.0.1:20121",
  });
  let results;
  try {
    results = await client.default.getCheck({
      nodeUrl: "http://127.0.0.1",
      baselineConfigurationId: "local_testnet",
    });
  } catch (e) {
    if (e instanceof ApiError) {
      console.log(e.body);
    }
    throw e;
  }
}, 30000);

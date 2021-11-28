// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import { assertEquals } from "https://deno.land/std@0.85.0/testing/asserts.ts";
import { consoleContext, UserContext } from "../main/context.ts";

Deno.test("UserContext.fromDisk", async () => {
  // "test" matches username created in rust test harness
  const username = "test";
  const testUser = await UserContext.fromDisk(username);
  assertEquals(testUser.username, username);

  const address = await Deno.readTextFile(
    consoleContext.accountAddressPath(username),
  );
  assertEquals(testUser.address, address);
  assertEquals(
    testUser.privateKeyPath,
    consoleContext.accountKeyPath(username),
  );
});

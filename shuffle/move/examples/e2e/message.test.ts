// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
//
// This file is generated on new project creation.

// deno-lint-ignore-file no-explicit-any
import { assert, assertExists } from "https://deno.land/std/testing/asserts.ts";
import * as Shuffle from "../repl.ts";
import * as utils from "./utils.ts";

Shuffle.test("Test Assert", () => {
  assert("Hello");
});

Shuffle.test("Ability to set message", async () => {
  const sender = Shuffle.senderAddress;
  assert(utils.deployMessageModule());
  const resources = await Shuffle.resources(sender);
  const messageResource = getMessageResource(resources);
  assertExists(messageResource);
});

function getMessageResource(resources: any[]) {
  for(const res of resources) {
    if(res["type"]["name"] == "Message") {
      return res;
    }
  }
  return null;
}

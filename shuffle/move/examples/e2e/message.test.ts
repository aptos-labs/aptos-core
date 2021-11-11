// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
//
// This file is generated on new project creation.

import {
  assert,
  fail,
} from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as DiemHelpers from "../main/helpers.ts";
import * as context from "../main/context.ts";
import * as devapi from "../main/devapi.ts";
import * as main from "../main/mod.ts";

Deno.test("Test Assert", () => {
  assert("Hello");
});

Deno.test("Ability to set message", async () => {
  console.log("Test sender address: " + context.senderAddress);
  console.log("Test receiver address: " + context.receiverAddress);
  await main.setMessageScriptFunction(
    "hello blockchain",
  );

  for (let i = 0; i < 10; i++) {
    const messageResource = (await devapi.resourcesWithName("MessageHolder"))[0];
    if (messageResource !== undefined) {
      const result = DiemHelpers.hexToAscii(messageResource["value"]["message"])
        .toString() === "\x00hello blockchain";
      if (result) {
        return;
      }
    }
    await new Promise((r) => setTimeout(r, 1000));
  }

  fail("Message was not set properly");
});

// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
//
// This file is generated on new project creation.

// deno-lint-ignore-file no-explicit-any
import { assert, assertEquals, fail } from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as DiemHelpers from "../main/helpers.ts";
import * as main from "../main/mod.ts";
import * as Shuffle from "../repl.ts";

Shuffle.test("Test Assert", () => {
  assert("Hello");
});

Shuffle.test("Ability to set message", async () => {
  const sender = Shuffle.senderAddress;
  console.log("Test sender address: " + sender);
  await main.setMessage("hello blockchain", (await Shuffle.sequenceNumber())!.valueOf());

  for (let i = 0; i < 10; i++) {
    const resources = await Shuffle.resources(sender);
    const messageResource = main.messagesFrom(resources)[0];
    if (messageResource !== undefined) {
      var result = DiemHelpers.hexToAscii(messageResource["value"]["message"]).toString() === "\x00hello blockchain";
      if (result) {
        return;
      }
    }
    await new Promise(r => setTimeout(r, 1000));
  }

  fail("Message was not set properly");
});

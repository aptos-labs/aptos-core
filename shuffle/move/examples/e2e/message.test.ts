// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
//
// This file is generated on new project creation.

import {
  assert,
  fail,
} from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as DiemHelpers from "../main/helpers.ts";
import * as main from "../main/mod.ts";
import * as Shuffle from "../repl.ts";

Shuffle.test("Test Assert", () => {
  assert("Hello");
});

Shuffle.test("Ability to set message", async () => {
  const sender = Shuffle.senderAddress;
  console.log("Test sender address: " + sender);
  const receiver = Shuffle.receiverAddress;
  console.log("Test receiver address: " + receiver);
  await main.setMessageScriptFunction(
    "hello blockchain",
    (await Shuffle.sequenceNumber())!.valueOf(),
  );

  for (let i = 0; i < 10; i++) {
    const resources = await Shuffle.resources(sender);
    const messageResource = main.resourcesWithName(resources, "MessageHolder")[0];
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

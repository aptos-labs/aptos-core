// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
//
// This file is generated on new project creation.

import {
  assert,
  assertEquals,
} from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as devapi from "../main/devapi.ts";
import * as helpers from "../main/helpers.ts";
import * as main from "../main/mod.ts";
import { defaultUserContext, UserContext } from "../main/context.ts";

Deno.test("Test Assert", () => {
  assert("Hello");
});

Deno.test("Ability to set message", async () => {
  let txn = await main.setMessageScriptFunction("hello blockchain");
  txn = await devapi.waitForTransactionCompletion(txn.hash);
  assert(txn.success);

  const expected = "hello blockchain";
  const messages = await main.decodedMessages();
  assertEquals(messages[0], expected);
});

Deno.test("Ability to set NFTs", async () => {
  const contentUri = "https://placekitten.com/200/300";
  let txn = await main.createTestNFTScriptFunction(contentUri);
  txn = await devapi.waitForTransactionCompletion(txn.hash);
  assert(txn.success);

  const uris = await main.decodedNFTs();
  assertEquals(uris[0], contentUri);
});

Deno.test("Advanced: Ability to set message from nonpublishing account", async () => {
  const publishingAddress = defaultUserContext.address;
  const scriptFunction = publishingAddress + "::Message::set_message";

  const secondUserContext = UserContext.fromEnv("test");

  let txn = await helpers.invokeScriptFunctionForContext(
    secondUserContext,
    scriptFunction,
    [],
    ["invoked script function from nonpublishing account"],
  );
  txn = await devapi.waitForTransactionCompletion(txn.hash);
  assert(txn.success);

  const messages = await main.decodedMessages(secondUserContext.address);
  assertEquals(
    messages[0],
    "invoked script function from nonpublishing account",
  );
});

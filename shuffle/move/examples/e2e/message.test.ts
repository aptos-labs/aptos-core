// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
//
// This file is generated on new project creation.

import {
  assert,
  assertEquals,
} from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as devapi from "../main/devapi.ts";
import * as main from "../main/mod.ts";

Deno.test("Test Assert", () => {
  assert("Hello");
});

Deno.test("Ability to set message", async () => {
  const txn = await main.setMessageScriptFunction("hello blockchain");

  assert(await devapi.transactionSuccess(txn.hash)); // wait for txn to succeed

  const expected = "\x00hello blockchain"; // prefixed with \x00 bc of bcs encoding
  const messages = await main.decodedMessages();
  assertEquals(messages[0], expected);
});

Deno.test("Ability to set NFTs", async () => {
  const contentUri = "https://placekitten.com/200/300";
  const txn = await main.createTestNFTScriptFunction(contentUri);

  assert(await devapi.transactionSuccess(txn.hash)); // wait for txn to succeed

  const uris = await main.decodedNFTs();
  assertEquals(uris[0], "\x00" + contentUri);
});

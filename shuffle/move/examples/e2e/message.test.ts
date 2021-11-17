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
import * as context from "../main/context.ts";
import * as helpers from "../main/helpers.ts";

Deno.test("Test Assert", () => {
  assert("Hello");
});

Deno.test("Ability to set message", async () => {
  let txn = await main.setMessageScriptFunction("hello blockchain");
  txn = await devapi.waitForTransactionCompletion(txn.hash);
  assert(txn.success);

  const expected = "hello blockchain"; // prefixed with \x00 bc of BCS encoding
  const messages = await main.decodedMessages();
  assertEquals(messages[0], expected);
});

Deno.test("Ability to set NFTs", async () => {
  let initialize_txn = await main.initializeNFTScriptFunction();
  initialize_txn = await devapi.waitForTransactionCompletion(initialize_txn.hash);
  assert(initialize_txn.success);

  const contentUri = "https://placekitten.com/200/300";
  let txn = await main.createTestNFTScriptFunction(contentUri);
  txn = await devapi.waitForTransactionCompletion(txn.hash);
  assert(txn.success);

  let resource = await devapi.resourcesWithName("NFT");
  console.log(resource);

  const nfts = await main.decodedNFTs();
  assertEquals(helpers.hexToAscii(nfts[0].content_uri), contentUri);

  console.log(nfts[0].id.id.addr);
  console.log(nfts[0].id.id.creation_num);
  const creator = nfts[0].id.id.addr;
  const creation_num = nfts[0].id.id.creation_num;

  let transfer_txn = await main.transferNFTScriptFunction(context.senderAddress, creator, creation_num);
  transfer_txn = await devapi.waitForTransactionCompletion(transfer_txn.hash);
  console.log(transfer_txn);
  assert(transfer_txn.success);
  const result = await main.decodedNFTs();
  assertEquals(result[0], contentUri);
});

Deno.test("Advanced: Ability to set message from nonpublishing account", async () => {
  const publishingAddress = context.defaultUserContext.address;
  const scriptFunction = publishingAddress + "::Message::set_message";

  const secondUserContext = context.UserContext.fromEnv("test");

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

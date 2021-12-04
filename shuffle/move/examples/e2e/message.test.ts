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

Deno.test("Test Assert", () => {
  assert("Hello");
});

Deno.test("Ability to set message", async () => {
  let txn = await main.setMessageScriptFunction("hello blockchain");
  txn = await devapi.waitForTransaction(txn.hash);
  assert(txn.success);

  const expected = "hello blockchain";
  const messages = await main.decodedMessages();
  assertEquals(messages[0], expected);

  txn = await main.setMessageScriptFunction("hello again");
  txn = await devapi.waitForTransaction(txn.hash);
  assert(txn.success);

  const events = await main.messageEvents();
  // In case there is another test also set message,
  // we only look for last event
  assert(events.length >= 1);
  const event = events[events.length - 1];
  assertEquals(event.data, {
    "from_message": "hello blockchain",
    "to_message": "hello again"
  });
});

Deno.test("Ability to set NFTs", async () => {
  // Initialize nft_collection resource for both sender and receiver
  let senderInitializeTxn = await main.initializeNFTScriptFunction(
    "TestNFT",
    context.defaultUserContext,
    context.defaultUserContext.address,
  );
  senderInitializeTxn = await devapi.waitForTransaction(
    senderInitializeTxn.hash,
  );
  const secondUserContext = context.UserContext.fromEnv("test");
  let receiverInitializeTxn = await main.initializeNFTScriptFunction(
    "TestNFT",
    secondUserContext,
    context.defaultUserContext.address,
  );
  receiverInitializeTxn = await devapi.waitForTransaction(
    receiverInitializeTxn.hash,
  );
  assert(senderInitializeTxn.success);
  assert(receiverInitializeTxn.success);

  // Mint TestNFT into sender address
  const contentUri = "https://placekitten.com/200/300";
  let txn = await main.createTestNFTScriptFunction(
    contentUri,
    "TestNFT",
    context.defaultUserContext,
    context.defaultUserContext.address,
  );
  txn = await devapi.waitForTransaction(txn.hash);
  assert(txn.success);

  const nfts = await main.decodedNFTs(context.defaultUserContext.address);
  assertEquals(nfts[0].content_uri, contentUri);

  // Transfer TestNFT from sender to receiver
  const creator = nfts[0].id.id.addr;
  const creationNum = nfts[0].id.id.creation_num;

  let transferTxn = await main.transferNFTScriptFunction(
    secondUserContext.address,
    creator,
    creationNum,
    "TestNFT",
    context.defaultUserContext,
    context.defaultUserContext.address,
  );
  transferTxn = await devapi.waitForTransaction(transferTxn.hash);
  assert(transferTxn.success);

  // Check receiver has the nft
  const receiverNFTs = await main.decodedNFTs(secondUserContext.address);
  assertEquals(receiverNFTs[0].content_uri, contentUri);
  // Check sender nft_collection is empty
  const senderNFTs = await main.decodedNFTs(
    context.defaultUserContext.address,
  );
  assert(senderNFTs.length === 0);
});

Deno.test("Advanced: Ability to set message from nonpublishing account", async () => {
  const secondUserContext = context.UserContext.fromEnv("test");
  let txn = await main.setMessageScriptFunction(
    "invoked script function from nonpublishing account",
    secondUserContext,
  );
  txn = await devapi.waitForTransaction(txn.hash);
  assert(txn.success);

  const messages = await main.decodedMessages(secondUserContext.address);
  assertEquals(
    messages[0],
    "invoked script function from nonpublishing account",
  );
});

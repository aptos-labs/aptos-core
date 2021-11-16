// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as DiemHelpers from "./helpers.ts";
import * as DiemTypes from "./generated/diemTypes/mod.ts";
import * as codegen from "./generated/diemStdlib/mod.ts";
import * as context from "./context.ts";
import * as devapi from "./devapi.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";
import { green } from "https://deno.land/x/nanocolors@0.1.12/mod.ts";

const textEncoder = new util.TextEncoder();

await printWelcome();

function highlight(content: string) {
  return green(content);
}

export async function printWelcome() {
  console.log(`Loading Project ${highlight(context.projectPath)}`);
  console.log(`Sender Account Address ${highlight(context.senderAddress)}`);
  console.log(
    `"helpers", "devapi", "context", "main", "codegen", "help" top level objects available`,
  );
  console.log(`Run "help" for more information on top level objects`);
  console.log(`Connecting to Node ${highlight(context.nodeUrl)}`);
  console.log(await devapi.ledgerInfo());
  console.log();
}

// ScriptFunction example; client side creation and signing of transactions.
// https://github.com/diem/diem/blob/main/json-rpc/docs/method_submit.md#method-submit
export async function setMessageScriptFunction(
  message: string,
) {
  const payload = codegen.Stdlib.encodeSetMessageScriptFunction(
    textEncoder.encode(message),
  );
  return await DiemHelpers.buildAndSubmitTransaction(
    context.senderAddress,
    await devapi.sequenceNumber(),
    context.privateKey(),
    payload,
  );
}

// Script example; client side creation and signing of transactions.
// https://github.com/diem/diem/blob/main/json-rpc/docs/method_submit.md#method-submit
export async function setMessageScript(
  message: string,
) {
  const script = codegen.Stdlib.encodeSetMessageScript(
    textEncoder.encode(message),
  );
  const payload = new DiemTypes.TransactionPayloadVariantScript(script);
  return await DiemHelpers.buildAndSubmitTransaction(
    context.senderAddress,
    await devapi.sequenceNumber(),
    context.privateKey(),
    payload,
  );
}

// Script example; initializes TestNFT utilizing the NFT<Type>
// generic methods. This example replaces the genesis initialize functionality
// but with a different address. See main/sources/NFT.move
// This is optional, as createTestNFTScriptFunction handles init.
export async function initializeTestNFT() {
  const nftAddress = DiemHelpers.hexToAccountAddress(context.senderAddress);

  // Create the type tag representing TestNFT to pass to the generic
  // script `initialize_nft`
  const testNftType = new DiemTypes.TypeTagVariantStruct(
    new DiemTypes.StructTag(
      nftAddress,
      new DiemTypes.Identifier("TestNFT"),
      new DiemTypes.Identifier("TestNFT"),
      [],
    ),
  );

  const script = codegen.Stdlib.encodeInitializeNftScript(testNftType);
  const payload = new DiemTypes.TransactionPayloadVariantScript(script);
  return await DiemHelpers.buildAndSubmitTransaction(
    context.senderAddress,
    await devapi.sequenceNumber(),
    context.privateKey(),
    payload,
  );
}

// ScriptFunction example; creation of NFT. Can only create one per account atm.
// See main/source/TestNFT.move
export async function createTestNFTScriptFunction(
  contentUri: string,
) {
  const payload = codegen.Stdlib.encodeCreateNftScriptFunction(
    textEncoder.encode(contentUri),
  );
  return await DiemHelpers.buildAndSubmitTransaction(
    context.senderAddress,
    await devapi.sequenceNumber(),
    context.privateKey(),
    payload,
  );
}

export async function decodedMessages() {
  return (await devapi.resourcesWithName("MessageHolder"))
    .map((entry) => DiemHelpers.hexToAscii(entry.value.message));
}

export async function decodedNFTs() {
  return (await devapi.resourcesWithName("NFT"))
    .filter((entry) => entry.value && entry.value.content_uri)
    .map((entry) => DiemHelpers.hexToAscii(entry.value.content_uri));
}

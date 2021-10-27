// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as DiemHelpers from "./helpers.ts";
import * as DiemTypes from "./generated/diemTypes/mod.ts";
import * as codegen from "./generated/diemStdlib/mod.ts";
import * as path from "https://deno.land/std@0.110.0/path/mod.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";

const textEncoder = new util.TextEncoder();
export const shuffleDir = Deno.env.get("SHUFFLE_HOME") || "unknown";
const privateKeyPath = path.join(shuffleDir, "accounts/latest/dev.key");
const senderAddressPath = path.join(shuffleDir, "accounts/latest/address");
const senderAddress = await Deno.readTextFile(senderAddressPath);
export const fullSenderAddress = "0x" + senderAddress;

// ScriptFunction example; client side creation and signing of transactions.
// https://github.com/diem/diem/blob/main/json-rpc/docs/method_submit.md#method-submit
export async function setMessageScriptFunction(
  message: string,
  sequenceNumber: number,
) {
  const privateKeyBytes = await Deno.readFile(privateKeyPath);
  const payload = codegen.Stdlib.encodeSetMessageScriptFunction(
    textEncoder.encode(message),
  );
  return await DiemHelpers.buildAndSubmitTransaction(
    fullSenderAddress,
    sequenceNumber,
    privateKeyBytes,
    payload,
  );
}

// Script example; client side creation and signing of transactions.
// https://github.com/diem/diem/blob/main/json-rpc/docs/method_submit.md#method-submit
export async function setMessageScript(
  message: string,
  sequenceNumber: number,
) {
  const privateKeyBytes = await Deno.readFile(privateKeyPath);
  const script = codegen.Stdlib.encodeSetMessageScript(
    textEncoder.encode(message),
  );
  const payload = new DiemTypes.TransactionPayloadVariantScript(script);
  return await DiemHelpers.buildAndSubmitTransaction(
    fullSenderAddress,
    sequenceNumber,
    privateKeyBytes,
    payload,
  );
}

// Script example; initializes TestNFT utilizing the NFT<Type>
// generic methods. This example replaces the genesis initialize functionality
// but with a different address. See main/sources/NFT.move
// This is optional, as createTestNFTScriptFunction handles init.
export async function initializeTestNFT(sequenceNumber: number) {
  const privateKeyBytes = await Deno.readFile(privateKeyPath);
  const nftAddress = DiemHelpers.hexToAccountAddress(senderAddress);

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
    fullSenderAddress,
    sequenceNumber,
    privateKeyBytes,
    payload,
  );
}

// ScriptFunction example; creation of NFT. Can only create one per account atm.
// See main/source/TestNFT.move
export async function createTestNFTScriptFunction(
  contentUri: string,
  sequenceNumber: number,
) {
  const privateKeyBytes = await Deno.readFile(privateKeyPath);
  const payload = codegen.Stdlib.encodeCreateNftScriptFunction(
    textEncoder.encode(contentUri),
  );
  return await DiemHelpers.buildAndSubmitTransaction(
    fullSenderAddress,
    sequenceNumber,
    privateKeyBytes,
    payload,
  );
}

// deno-lint-ignore no-explicit-any
export function resourcesWithName(resources: any[], resourceName: string) {
  return resources
    .filter(
      (entry) => entry["type"]["name"] == resourceName,
    );
}

// deno-lint-ignore no-explicit-any
export function decodedMessages(resources: any[]) {
  return resourcesWithName(resources, "MessageHolder")
    .map((entry) => DiemHelpers.hexToAscii(entry.value.message));
}

// deno-lint-ignore no-explicit-any
export function decodedNFTs(resources: any[]) {
  return resourcesWithName(resources, "NFT")
    .map((entry) => DiemHelpers.hexToAscii(entry.value.content_uri));
}

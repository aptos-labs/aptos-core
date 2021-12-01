// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// deno-lint-ignore-file no-explicit-any
import * as DiemHelpers from "./helpers.ts";
import * as DiemTypes from "./generated/diemTypes/mod.ts";
import * as codegen from "./generated/diemStdlib/mod.ts";
import {
  addressOrDefault,
  consoleContext,
  defaultUserContext,
  UserContext,
} from "./context.ts";
import * as devapi from "./devapi.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";
import { green } from "https://deno.land/x/nanocolors@0.1.12/mod.ts";

const textEncoder = new util.TextEncoder();

await printWelcome();

function highlight(content: string) {
  return green(content);
}

export async function printWelcome() {
  console.log(`Loading Project ${highlight(consoleContext.projectPath)}`);
  console.log(
    `Default Account Address ${highlight(defaultUserContext.address)}`,
  );
  console.log(
    `"helpers", "devapi", "context", "main", "codegen", "help" top level objects available`,
  );
  console.log(`Run "help" for more information on top level objects`);
  console.log(
    `Connecting to ${consoleContext.networkName} at ${
      highlight(consoleContext.client.baseUrl)
    }`,
  );
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
    defaultUserContext.address,
    await devapi.sequenceNumber(),
    await defaultUserContext.readPrivateKey(),
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
    defaultUserContext.address,
    await devapi.sequenceNumber(),
    await defaultUserContext.readPrivateKey(),
    payload,
  );
}

// Script example; initializes TestNFT utilizing the NFT<Type>
// generic methods. This example replaces the genesis initialize functionality
// but with a different address. See main/sources/NFT.move
// This is optional, as createTestNFTScriptFunction handles init.
export async function initializeTestNFT() {
  const nftAddress = DiemHelpers.hexToAccountAddress(
    defaultUserContext.address,
  );

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

  const payload = codegen.Stdlib.encodeInitializeNftCollectionScriptFunction(
    testNftType,
  );
  return await DiemHelpers.buildAndSubmitTransaction(
    defaultUserContext.address,
    await devapi.sequenceNumber(),
    await defaultUserContext.readPrivateKey(),
    payload,
  );
}

// ScriptFunction example; creation of NFT.
// See main/source/TestNFT.move
export async function createTestNFTScriptFunction(
  contentUri: string,
) {
  const scriptFunction: string = defaultUserContext.address +
    "::TestNFT::create_nft";
  const typeArguments: string[] = [];
  const args: any[] = [contentUri];
  return await DiemHelpers.invokeScriptFunction(
    scriptFunction,
    typeArguments,
    args,
  );
}

// ScriptFunction example; creation of NFT.
// See main/source/TestNFT.move
export async function transferNFTScriptFunction(
  to: string,
  creator: string,
  creationNum: number,
) {
  const scriptFunction: string = defaultUserContext.address +
    "::NFTStandard::transfer";

  const typeArgument = defaultUserContext.address + "::TestNFT::TestNFT";
  const typeArguments: string[] = [typeArgument];

  const args: any[] = [to, creator, creationNum];
  return await DiemHelpers.invokeScriptFunction(
    scriptFunction,
    typeArguments,
    args,
  );
}

export async function initializeNFTScriptFunction(userContext: UserContext) {
  const scriptFunction: string = defaultUserContext.address +
    "::NFTStandard::initialize_nft_collection";

  const typeArgument = defaultUserContext.address + "::TestNFT::TestNFT";
  const typeArguments: string[] = [typeArgument];

  const args: any[] = [];
  return await DiemHelpers.invokeScriptFunctionForContext(
    userContext,
    scriptFunction,
    typeArguments,
    args,
  );
}

export async function decodedMessages(addr?: string) {
  addr = addressOrDefault(addr);
  return (await devapi.resourcesWithName("MessageHolder", addr))
    .map((entry) => DiemHelpers.hexToAscii(entry.data.message));
}

export async function decodedNFTs(addr?: string) {
  addr = addressOrDefault(addr);
  const decodedNfts: any[] = [];
  const nfts = (await devapi.resourcesWithName("NFTStandard", addr))
    .filter((entry) => entry.data && entry.data.nfts)
    .map((entry) => {
      return entry.data.nfts;
    });
  nfts.forEach((nftType: any) => {
    nftType.forEach((nft: any) => {
      decodedNfts.push(nft);
    });
  });
  return decodedNfts;
}

// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// deno-lint-ignore-file no-explicit-any
// deno-lint-ignore-file ban-types

import * as DiemHelpers from "./helpers.ts";
import {
  addressOrDefault,
  consoleContext,
  defaultUserContext,
  UserContext,
} from "./context.ts";
import * as devapi from "./devapi.ts";
import * as mv from "./move.ts";
import { green } from "https://deno.land/x/nanocolors@0.1.12/mod.ts";

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

// Invoke SetMessage script function by creating and executing transaction
// with set_message script function payload.
// See main/source/Message.move
export async function setMessageScriptFunction(
  message: string,
  sender?: UserContext,
  moduleAddress?: string,
) {
  return await invokeScriptFunction(
    "Message::set_message",
    [],
    [mv.Ascii(message)],
    sender,
    moduleAddress,
  );
}

// ScriptFunction example; creation of NFT.
// See main/source/nft/TestNFT.move
export async function createTestNFTScriptFunction(
  contentUri: string,
  sender?: UserContext,
  moduleAddress?: string,
) {
  return await invokeScriptFunction(
    "TestNFT::create_nft",
    [],
    [mv.Ascii(contentUri)],
    sender,
    moduleAddress,
  );
}

// ScriptFunction example; creation of NFT.
// See main/source/TestNFT.move
export async function transferNFTScriptFunction(
  to: string,
  creator: string,
  creationNum: string,
  sender?: UserContext,
  moduleAddress?: string,
) {
  moduleAddress = moduleAddress || defaultUserContext.address;

  return await invokeScriptFunction(
    "NFTStandard::transfer",
    [`${moduleAddress}::TestNFT::TestNFT`],
    [mv.Address(to), mv.Address(creator), mv.U64(creationNum)],
    sender,
    moduleAddress,
  );
}

// Initialize NFT
// See main/source/nft/NFTStandard.move
// See main/source/nft/TestNFT.move
export async function initializeNFTScriptFunction(
  sender?: UserContext,
  moduleAddress?: string,
) {
  moduleAddress = moduleAddress || defaultUserContext.address;

  return await invokeScriptFunction(
    "NFTStandard::initialize_nft_collection",
    [`${moduleAddress}::TestNFT::TestNFT`],
    [],
    sender,
    moduleAddress,
  );
}

async function invokeScriptFunction(
  funcName: string,
  typeArgs: string[],
  args: mv.MoveType[],
  sender?: UserContext,
  moduleAddress?: string,
) {
  sender = sender || defaultUserContext;
  moduleAddress = moduleAddress || defaultUserContext.address;

  return await DiemHelpers.invokeScriptFunctionForContext(
    sender,
    `${moduleAddress}::${funcName}`,
    typeArgs,
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
  nfts.forEach((nft_type: any) => {
    nft_type.forEach((nft: any) => {
      decodedNfts.push({
        id: nft.id,
        content_uri: DiemHelpers.hexToAscii(nft.content_uri),
      });
    });
  });
  return decodedNfts;
}

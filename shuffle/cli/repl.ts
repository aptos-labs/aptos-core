// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// Generated on new project creation. Invoked by shuffle CLI.

// Creates typescript wrappers around the Developer API for easier consumption,
// including endpoints: ledgerInfo, resources, modules, and some of transactions.
// Developer API: https://docs.google.com/document/d/1KEPnGGU3zg_RmN8V4r2ms_MFPwsTMNyK7jCUFygviDg/edit#heading=h.hesw425dw9gz

// deno-lint-ignore-file no-explicit-any
import * as path from "https://deno.land/std@0.110.0/path/mod.ts";
import { green } from 'https://deno.land/x/nanocolors@0.1.12/mod.ts';
import { createRemote } from "https://deno.land/x/gentle_rpc@v3.1/mod.ts";
import urlcat from 'https://deno.land/x/urlcat@v2.0.4/src/index.ts';
import { isURL } from "https://deno.land/x/is_url@v1.0.1/mod.ts";

function highlight(content: string) {
  return green(content);
}

export const shuffleDir = String(Deno.env.get("SHUFFLE_HOME"));
export const projectPath = String(Deno.env.get("PROJECT_PATH"));
export const nodeUrl = getNetworkEndpoint(String(Deno.env.get("SHUFFLE_NETWORK")));
export const senderAddress = String(Deno.env.get("SENDER_ADDRESS"));
export const privateKeyPath = String(Deno.env.get("PRIVATE_KEY_PATH"));

export const receiverPrivateKeyPath = path.join(shuffleDir, "accounts/test/dev.key");
export const receiverAddressPath = path.join(shuffleDir, "accounts/test/address");
export const receiverAddress = await Deno.readTextFile(receiverAddressPath);

console.log(`Loading Project ${highlight(projectPath)}`);
console.log(`Connected to Node ${highlight(nodeUrl)}`);
console.log(`Sender Account Address ${highlight(senderAddress)}`);
console.log(`"Shuffle", "main", "codegen", "DiemHelpers", "help" top level objects available`);
console.log(`Run "help" for more information on top level objects`);
console.log(await ledgerInfo());
console.log();

export async function ledgerInfo() {
  const res = await fetch(relativeUrl("/"));
  return await res.json();
}

export async function transactions() {
  const res = await fetch(relativeUrl("/transactions"));
  return await res.json();
}

export async function accountTransactions() {
  const remote = createRemote("http://127.0.0.1:8080/v1");
  return await remote.call(
      "get_account_transactions",
      [senderAddress.substring(2), 0, 10, true]
  );
}

export async function resources(addr: string | undefined) {
  if(addr === undefined) {
    addr = senderAddress;
  }
  const res = await fetch(relativeUrl(`/accounts/${addr}/resources`));
  return await res.json();
}

export async function modules(addr: string | undefined) {
  if(addr === undefined) {
    addr = senderAddress;
  }
  const res = await fetch(relativeUrl(`/accounts/${addr}/modules`));
  return await res.json();
}

// Gets the sender address's account resource from the developer API.
// Example payload below:
// {
//   "type": "0x1::DiemAccount::DiemAccount",
//   "value": {
//     "sequence_number": "2",
export async function account() {
  const res = await resources(senderAddress);
  return res.
  find(
      (entry: any) => entry["type"] == "0x1::DiemAccount::DiemAccount"
  );
}

export async function sequenceNumber() {
  const acc = await account();
  if (acc) {
    return parseInt(acc["value"]["sequence_number"]);
  }
  return null;
}

export async function accounts() {
  return [await account()];
}

export const test = Deno.test;

function relativeUrl(tail: string) {
  return new URL(tail, nodeUrl).href;
}

function getNetworkEndpoint(inputNetwork : string) {
  if (inputNetwork == "unknown") {
    throw new Error("Invalid network.")
  }
  let network = "";
  if (isURL(inputNetwork)) {
    network = inputNetwork;
  } else {
    network = urlcat("http://",inputNetwork);
  }
  return network;
}

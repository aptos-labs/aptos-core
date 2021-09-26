// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// Creates typescript wrappers around the Developer API for easier consumption,
// including endpoints: ledgerInfo, resources, modules, and some of transactions.
// Developer API: https://docs.google.com/document/d/1KEPnGGU3zg_RmN8V4r2ms_MFPwsTMNyK7jCUFygviDg/edit#heading=h.hesw425dw9gz

import { green } from 'https://deno.land/x/nanocolors/mod.ts';

function highlight(content: string) {
  return green(content);
}

// TODO: Replace all hardcoding with calculated or env retrieved values.
export const projectPath = Deno.env.get("PROJECT_PATH") || "unknown";
export const nodeUrl = 'http://127.0.0.1:8081'
export const senderAddress = "0x24163afcc6e33b0a9473852e18327fa9";

console.log(`Loading REPL for project ${highlight(projectPath)}`);
console.log(`Connected to Node ${highlight(nodeUrl)}`);
console.log(`Sender Account Address ${highlight(senderAddress)}`);
console.log(`"Shuffle", "TxnBuilder" top level objects available`);
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

export async function accounts() {
  return [{ "address": senderAddress }];
}

function relativeUrl(tail: string) {
  return new URL(tail, nodeUrl).href;
}

// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// Generated on new project creation. Invoked by shuffle CLI.

// Creates typescript wrappers around the Developer API for easier consumption,
// including endpoints: ledgerInfo, resources, modules, and some of transactions.
// Developer API: https://docs.google.com/document/d/1KEPnGGU3zg_RmN8V4r2ms_MFPwsTMNyK7jCUFygviDg/edit#heading=h.hesw425dw9gz

// deno-lint-ignore-file no-explicit-any
// deno-lint-ignore-file ban-types
import { createRemote } from "https://deno.land/x/gentle_rpc@v3.1/mod.ts";
import * as context from "./context.ts";

console.log(await ledgerInfo());

export async function ledgerInfo() {
  return await checkingFetch(context.relativeUrl("/"));
}

export async function transactions() {
  return await checkingFetch(context.relativeUrl("/transactions"));
}

export async function accountTransactions() {
  const remote = createRemote("http://127.0.0.1:8080/v1");
  return await remote.call(
    "get_account_transactions",
    [context.senderAddress.substring(2), 0, 10, true],
  );
}

// deno-lint-ignore ban-types
export async function resources(addr?: string): Promise<object[]> {
  addr = context.addressOrDefault(addr);
  return await checkingFetch(
    context.relativeUrl(`/accounts/${addr}/resources`),
  );
}

export async function modules(addr?: string) {
  addr = context.addressOrDefault(addr);
  return await checkingFetch(context.relativeUrl(`/accounts/${addr}/modules`));
}

// Gets the sender address's account resource from the developer API.
// Example payload below:
// {
//   "type": "0x1::DiemAccount::DiemAccount",
//   "value": {
//     "sequence_number": "2",
export async function account(addr?: string) {
  addr = context.addressOrDefault(addr);
  const res = await resources(addr);
  return res
    .find(
      (entry: any) => entry["type"] == "0x1::DiemAccount::DiemAccount",
    );
}

export async function sequenceNumber(addr?: string): Promise<number> {
  const acc: any = await account(addr);
  if (acc) {
    return parseInt(acc["value"]["sequence_number"]);
  }
  throw "unable to find account";
}

export async function accounts() {
  return [await account()];
}

export async function postTransactionBcs(
  body: string | Uint8Array,
): Promise<any> {
  const settings = {
    method: "POST",
    body: body,
    headers: {
      "Content-Type": "application/vnd.bcs+signed_transaction",
    },
  };
  return await checkingFetch(context.relativeUrl("/transactions"), settings);
}

export async function postTransactionSigningMessage(
  body: string,
): Promise<any> {
  return await checkingFetch(
    context.relativeUrl("/transactions/signing_message"),
    {
      method: "POST",
      body: body,
      headers: {
        "Content-Type": "application/json",
      },
    },
  );
}

export async function postTransactionJson(body: string): Promise<any> {
  return await checkingFetch(context.relativeUrl("/transactions"), {
    method: "POST",
    body: body,
    headers: {
      "Content-Type": "application/json",
    },
  });
}

export async function resourceNames(): Promise<string[]> {
  return (await resources())
    .map(
      (entry: any) => entry["type"],
    );
}

export async function resourcesWithName(resourceName: string): Promise<any[]> {
  return (await resources())
    .filter(
      (entry: any) => entry["type"].split("::").includes(resourceName),
    );
}

async function checkingFetch(
  relativePath: string,
  settings?: object,
): Promise<any> {
  const res = await fetch(context.relativeUrl(relativePath), settings);
  if (!isSuccess(res.status)) {
    try {
      const payload = await res.json();
      throw `error with fetch: ${res.statusText}. ${payload}`;
    } catch (_e) {
      throw `error with fetch: ${res.statusText}.`;
    }
  } else {
    return await res.json();
  }
}

function isSuccess(status: number) {
  return status >= 200 && status < 300;
}

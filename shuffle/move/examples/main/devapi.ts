// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// Generated on new project creation. Invoked by shuffle CLI.

// Creates typescript wrappers around the Developer API for easier consumption,
// including endpoints: ledgerInfo, resources, modules, and some of transactions.
// Developer API: https://docs.google.com/document/d/1KEPnGGU3zg_RmN8V4r2ms_MFPwsTMNyK7jCUFygviDg/edit#heading=h.hesw425dw9gz

// deno-lint-ignore-file no-explicit-any
// deno-lint-ignore-file ban-types
import * as context from "./context.ts";
import { delay } from "https://deno.land/std@0.114.0/async/delay.ts";

// Retrieves the ledger information as defined by the root /
// of the Developer API
export async function ledgerInfo() {
  return await checkingFetch(context.relativeUrl("/"));
}

// Returns a list of transactions, ascending from page 0.
export async function transactions() {
  // TODO: Have below return a list of transactions desc by sequence number
  return await checkingFetch(context.relativeUrl("/transactions"));
}

// Returns a specific transaction based on the version or hash.
export async function transaction(versionOrHash: string) {
  return await checkingFetch(
    context.relativeUrl(`/transactions/${versionOrHash}`),
  );
}

// Polls for a specific transaction to complete, returning the txn object.
export async function waitForTransactionCompletion(
  versionOrHash: string,
): Promise<any> {
  let txn = await transaction(versionOrHash);
  for (let i = 0; i < 20; i++) {
    if (txn.type !== "pending_transaction") {
      return txn;
    }
    await delay(500);
    txn = await transaction(versionOrHash);
  }
  throw `txn ${versionOrHash} never completed: ${txn && txn.vm_status}`;
}

// Returns transactions specific to a particular address.
export async function accountTransactions(addr?: string) {
  addr = context.addressOrDefault(addr);
  return await checkingFetch(
    context.relativeUrl(`/accounts/${addr}/transactions`),
  );
}

// Returns resources for a specific address.
// deno-lint-ignore ban-types
export async function resources(addr?: string): Promise<object[]> {
  addr = context.addressOrDefault(addr);
  return await checkingFetch(
    context.relativeUrl(`/accounts/${addr}/resources`),
  );
}

// Returns modules for a specific address, or the default account.
export async function modules(addr?: string) {
  addr = context.addressOrDefault(addr);
  return await checkingFetch(context.relativeUrl(`/accounts/${addr}/modules`));
}

// Gets the account resource for a particular adress, or the default account.
export async function account(addr?: string) {
  addr = context.addressOrDefault(addr);
  const res = await resources(addr);
  return res
    .find(
      (entry: any) => entry["type"] == "0x1::DiemAccount::DiemAccount",
    );
}

// Returns the sequence number for a particular address, or the default account
// for the console if no address is passed.
export async function sequenceNumber(addr?: string): Promise<number> {
  const acc: any = await account(addr);
  if (acc) {
    return parseInt(acc["data"]["sequence_number"]);
  }
  throw "unable to find account";
}

export async function accounts() {
  return [await account()];
}

// POSTs a BCS payload to the /transactions endpoint in the developer API.
export async function postTransactionBcs(
  body: string | Uint8Array,
): Promise<any> {
  const settings = {
    method: "POST",
    body: body,
    headers: {
      "Content-Type": "application/x.diem.signed_transaction+bcs",
    },
  };
  return await checkingFetch(context.relativeUrl("/transactions"), settings);
}

// POSTs a JSON payload to the /transactions/signing_message endpoint in the
// developer API to get the signing message for a payload.
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

// POSTs a JSON payload to the /transactions endpoint in the developer API.
export async function postTransactionJson(body: string): Promise<any> {
  return await checkingFetch(context.relativeUrl("/transactions"), {
    method: "POST",
    body: body,
    headers: {
      "Content-Type": "application/json",
    },
  });
}

export async function resourceNames(addr?: string): Promise<string[]> {
  return (await resources(addr))
    .map(
      (entry: any) => entry["type"],
    );
}

export async function resourcesWithName(
  resourceName: string,
  addr?: string,
): Promise<any[]> {
  return (await resources(addr))
    .filter(
      (entry: any) => entry["type"].split("::").includes(resourceName),
    );
}

async function checkingFetch(
  relativePath: string,
  // deno-lint-ignore ban-types
  settings?: object,
): Promise<any> {
  const res = await fetch(context.relativeUrl(relativePath), settings);
  if (!isSuccess(res.status)) {
    const payload = await res.json();
    throw `error with fetch: ${res.status} ${res.statusText}. ${payload.message}`;
  } else {
    return await res.json();
  }
}

function isSuccess(status: number) {
  return status >= 200 && status < 300;
}

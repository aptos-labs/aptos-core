// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import {
  assert,
  assertEquals,
  assertThrowsAsync,
} from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as devapi from "../main/devapi.ts";

Deno.test("ledgerInfo", async () => {
  const actual = await devapi.ledgerInfo();
  assertEquals(actual.chain_id, 4);
});

Deno.test("sequenceNumber", async () => {
  const actual = await devapi.sequenceNumber();
  assert(Number.isInteger(actual));
});

Deno.test("transactions", async () => {
  const actual = await devapi.transactions();
  assert(actual.length > 0);
  assert(actual[0].success);
});

Deno.test("transaction", async () => {
  const actual = await devapi.transaction(0);
  switch (actual.type) {
    case "genesis_transaction":
      assert(actual.success);
      break;
    default:
      throw "expect genesis_transaction for version 0";
  }
});

Deno.test("transaction not found", async () => {
  await assertThrowsAsync(async () => await devapi.transaction("invalid-hash"));
});

Deno.test("wait for txn complete", async () => {
  const txn = await devapi.waitForTransaction(0);
  assert(txn.success);

  await assertThrowsAsync(async () =>
    await devapi.waitForTransaction("invalid-hash")
  );
});

Deno.test("wait for txn timeout", async () => {
  const txnHash =
    "0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1";
  await assertThrowsAsync(
    async () => await devapi.waitForTransaction(txnHash, 300),
    Error,
    "timeout",
  );
});

Deno.test("accountTransactions", async () => {
  const actual = await devapi.accountTransactions();
  assert(Array.isArray(actual));
});

Deno.test("resources", async () => {
  const actual = await devapi.resources();
  assert(actual);

  const accounts = await devapi.resourcesWithName("DiemAccount");
  assert(accounts.length >= 1);
});

Deno.test("modules", async () => {
  const actual = await devapi.modules();
  assert(Array.isArray(actual));

  const modules = await devapi.modules("0x1");
  assert(Array.isArray(modules));
});

Deno.test("account", async () => {
  const actual = await devapi.account();
  assert(actual);
});

Deno.test("events", async () => {
  const handleStruct = "0x1::DiemAccount::AccountOperationsCapability";
  const accountAddress = "0xa550c18";
  const events = await devapi.events(handleStruct, "creation_events", undefined, undefined, accountAddress);
  // default limit is 25, we need at least 3 events for the following assertions
  assert(events.length > 3);
  // default start is 0
  assertEquals(events[0].sequence_number, "0");

  const events1 = await devapi.events(handleStruct, "creation_events", 0, 2, accountAddress);
  assertEquals(events1.length, 2);

  const events2 = await devapi.events(handleStruct, "creation_events", 1, 2, accountAddress);
  assertEquals(events2.length, 2);

  assertEquals(events1[1], events2[0]);
});

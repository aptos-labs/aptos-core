// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import {
  assert,
  assertEquals,
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
  assertEquals(actual.length, 1);
  assert(actual[0].success);
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

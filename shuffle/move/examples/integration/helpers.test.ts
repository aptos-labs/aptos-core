// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import {
  assert,
  assertEquals,
} from "https://deno.land/std@0.85.0/testing/asserts.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";
import { defaultUserContext } from "../main/context.ts";
import * as devapi from "../main/devapi.ts";
import * as mv from "../main/move.ts";
import * as helpers from "../main/helpers.ts";
import * as codegen from "../main/generated/diemStdlib/mod.ts";

Deno.test("invokeScriptFunction", async () => {
  const scriptFunction = defaultUserContext.address + "::Message::set_message";
  let txn = await helpers.invokeScriptFunction(
    scriptFunction,
    [],
    [mv.Ascii("invoked script function")],
  );
  txn = await devapi.waitForTransaction(txn.hash);
  assert(txn.success);

  assertEquals(txn.vm_status, "Executed successfully");
  assertEquals(txn.payload.function, scriptFunction);
  assertEquals(
    helpers.hexToAscii(txn.payload.arguments[0]),
    "invoked script function",
  );
});

Deno.test("buildAndSubmitTransaction with generated code", async () => {
  const textEncoder = new util.TextEncoder();
  const payload = codegen.Stdlib.encodeSetMessageScriptFunction(
    textEncoder.encode("hello world!"),
  );
  let txn = await helpers.buildAndSubmitTransaction(
    defaultUserContext.address,
    await devapi.sequenceNumber(),
    await defaultUserContext.readPrivateKey(),
    payload,
  );

  txn = await devapi.waitForTransaction(txn.hash);
  assert(txn.success);

  assertEquals(
    helpers.hexToAscii(txn.payload.arguments[0]),
    "hello world!",
  );
});

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { HexString } from "../hex_string";
import { Deserializer } from "../bcs";
import { ScriptABI, EntryFunctionABI, TransactionScriptABI } from "./abi";
import { TypeTagAddress, TypeTagU64 } from "./type_tag";

// eslint-disable-next-line operator-linebreak
const SCRIPT_FUNCTION_ABI =
  // eslint-disable-next-line max-len
  "010E6372656174655F6163636F756E740000000000000000000000000000000000000000000000000000000000000001074163636F756E7420204261736963206163636F756E74206372656174696F6E206D6574686F64732E000108617574685F6B657904";

// eslint-disable-next-line operator-linebreak
const TRANSACTION_SCRIPT_ABI =
  // eslint-disable-next-line max-len
  "00046D61696E0F20412074657374207363726970742E8B01A11CEB0B050000000501000403040A050E0B071924083D200000000101020301000003010400020C0301050001060C0101074163636F756E74065369676E65720A616464726573735F6F66096578697374735F617400000000000000000000000000000000000000000000000000000000000000010000010A0E0011000C020B021101030705090B0127020001016902";

describe("ABI", () => {
  it("parses create_acount successfully", async () => {
    const deserializer = new Deserializer(new HexString(SCRIPT_FUNCTION_ABI).toUint8Array());
    const entryFunctionABI = ScriptABI.deserialize(deserializer) as EntryFunctionABI;
    const { address: moduleAddress, name: moduleName } = entryFunctionABI.module_name;
    expect(entryFunctionABI.name).toBe("create_account");
    expect(HexString.fromUint8Array(moduleAddress.address).toShortString()).toBe("0x1");
    expect(moduleName.value).toBe("Account");
    expect(entryFunctionABI.doc.trim()).toBe("Basic account creation methods.");

    const arg = entryFunctionABI.args[0];
    expect(arg.name).toBe("auth_key");
    expect(arg.type_tag instanceof TypeTagAddress).toBeTruthy();
  });

  it("parses script abi successfully", async () => {
    const deserializer = new Deserializer(new HexString(TRANSACTION_SCRIPT_ABI).toUint8Array());
    const transactionScriptABI = ScriptABI.deserialize(deserializer) as TransactionScriptABI;
    expect(transactionScriptABI.name).toBe("main");
    expect(transactionScriptABI.doc.trim()).toBe("A test script.");

    expect(HexString.fromUint8Array(transactionScriptABI.code).hex()).toBe(
      // eslint-disable-next-line max-len
      "0xa11ceb0b050000000501000403040a050e0b071924083d200000000101020301000003010400020c0301050001060c0101074163636f756e74065369676e65720a616464726573735f6f66096578697374735f617400000000000000000000000000000000000000000000000000000000000000010000010a0e0011000c020b021101030705090b012702",
    );

    const arg = transactionScriptABI.args[0];
    expect(arg.name).toBe("i");
    expect(arg.type_tag instanceof TypeTagU64).toBeTruthy();
  });
});

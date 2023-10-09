// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Serializer, Deserializer } from "../../src/bcs";
import { AccountAddress } from "../../src/core";
import {
  ScriptTransactionArgument,
  ScriptTransactionArgumentAddress,
  ScriptTransactionArgumentBool,
  ScriptTransactionArgumentU128,
  ScriptTransactionArgumentU16,
  ScriptTransactionArgumentU256,
  ScriptTransactionArgumentU32,
  ScriptTransactionArgumentU64,
  ScriptTransactionArgumentU8,
  ScriptTransactionArgumentU8Vector,
} from "../../src/transactions/types";
import { Bool, U128, U16, U256, U32, U64, U8 } from "../../src/bcs/serializable/move-primitives";
import { MoveVector } from "../../src/bcs/serializable/move-structs";

describe("Tests for the script transaction argument class", () => {
  let serializer: Serializer;
  let scriptU8Bytes: Uint8Array;
  let scriptU16Bytes: Uint8Array;
  let scriptU32Bytes: Uint8Array;
  let scriptU64Bytes: Uint8Array;
  let scriptU128Bytes: Uint8Array;
  let scriptU256Bytes: Uint8Array;
  let scriptBoolBytes: Uint8Array;
  let scriptAddressBytes: Uint8Array;
  let scriptVectorU8Bytes: Uint8Array;

  beforeEach(() => {
    serializer = new Serializer();
    scriptU8Bytes = new Uint8Array([0, 1]);
    scriptU16Bytes = new Uint8Array([6, 2, 0]);
    scriptU32Bytes = new Uint8Array([7, 3, 0, 0, 0]);
    scriptU64Bytes = new Uint8Array([1, 4, 0, 0, 0, 0, 0, 0, 0]);
    scriptU128Bytes = new Uint8Array([2, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    scriptU256Bytes = new Uint8Array([
      8, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    scriptBoolBytes = new Uint8Array([5, 0]);
    scriptAddressBytes = new Uint8Array([3, ...AccountAddress.FOUR.data]);
    scriptVectorU8Bytes = new Uint8Array([4, 5, 1, 2, 3, 4, 5]);
  });

  it("should serialize all types of ScriptTransactionArguments correctly", () => {
    const validateBytes = (input: ScriptTransactionArgument, expectedOutput: Uint8Array) => {
      const serializer = new Serializer();
      input.serialize(serializer);
      const serializedBytes = serializer.toUint8Array();
      expect(serializedBytes).toEqual(expectedOutput);
    };
    validateBytes(new ScriptTransactionArgumentU8(1), scriptU8Bytes);
    validateBytes(new ScriptTransactionArgumentU16(2), scriptU16Bytes);
    validateBytes(new ScriptTransactionArgumentU32(3), scriptU32Bytes);
    validateBytes(new ScriptTransactionArgumentU64(4), scriptU64Bytes);
    validateBytes(new ScriptTransactionArgumentU128(5), scriptU128Bytes);
    validateBytes(new ScriptTransactionArgumentU256(6), scriptU256Bytes);
    validateBytes(new ScriptTransactionArgumentBool(false), scriptBoolBytes);
    validateBytes(new ScriptTransactionArgumentAddress(AccountAddress.FOUR), scriptAddressBytes);
    validateBytes(new ScriptTransactionArgumentU8Vector([1, 2, 3, 4, 5]), scriptVectorU8Bytes);
  });

  it("should deserialize all types of ScriptTransactionArguments correctly", () => {
    const deserializeToScriptArg = (input: ScriptTransactionArgument) => {
      const deserializer = new Deserializer(input.bcsToBytes());
      return ScriptTransactionArgument.deserialize(deserializer);
    };

    const scriptArgU8 = deserializeToScriptArg(new ScriptTransactionArgumentU8(1)) as ScriptTransactionArgumentU8;
    const scriptArgU16 = deserializeToScriptArg(new ScriptTransactionArgumentU16(2)) as ScriptTransactionArgumentU16;
    const scriptArgU32 = deserializeToScriptArg(new ScriptTransactionArgumentU32(3)) as ScriptTransactionArgumentU32;
    const scriptArgU64 = deserializeToScriptArg(new ScriptTransactionArgumentU64(4)) as ScriptTransactionArgumentU64;
    const scriptArgU128 = deserializeToScriptArg(new ScriptTransactionArgumentU128(5)) as ScriptTransactionArgumentU128;
    const scriptArgU256 = deserializeToScriptArg(new ScriptTransactionArgumentU256(6)) as ScriptTransactionArgumentU256;
    const scriptArgBool = deserializeToScriptArg(
      new ScriptTransactionArgumentBool(false),
    ) as ScriptTransactionArgumentBool;
    const scriptArgAddress = deserializeToScriptArg(
      new ScriptTransactionArgumentAddress(AccountAddress.FOUR),
    ) as ScriptTransactionArgumentAddress;
    const scriptArgU8Vector = deserializeToScriptArg(
      new ScriptTransactionArgumentU8Vector([1, 2, 3, 4, 5]),
    ) as ScriptTransactionArgumentU8Vector;

    expect(scriptArgU8.value.value).toEqual(1);
    expect(scriptArgU16.value.value).toEqual(2);
    expect(scriptArgU32.value.value).toEqual(3);
    expect(scriptArgU64.value.value).toEqual(4n);
    expect(scriptArgU128.value.value).toEqual(5n);
    expect(scriptArgU256.value.value).toEqual(6n);
    expect(scriptArgBool.value.value).toEqual(false);
    expect(scriptArgAddress.value.data).toEqual(AccountAddress.FOUR.data);
    expect(scriptArgU8Vector.value.values.map((v) => v.value)).toEqual([1, 2, 3, 4, 5]);
  });

  it("should convert all Move primitives to script transaction arguments correctly", () => {
    const deserializeToScriptArg = (
      input: U8 | U16 | U32 | U64 | U128 | U256 | Bool | MoveVector<U8> | AccountAddress,
    ) => {
      const scriptArg = ScriptTransactionArgument.fromMovePrimitive(input);
      serializer = new Serializer();
      scriptArg.serialize(serializer);
      const deserializer = new Deserializer(serializer.toUint8Array());
      return ScriptTransactionArgument.deserialize(deserializer);
    };

    expect(deserializeToScriptArg(new U8(1)) instanceof ScriptTransactionArgumentU8).toBe(true);
    expect(deserializeToScriptArg(new U16(2)) instanceof ScriptTransactionArgumentU16).toBe(true);
    expect(deserializeToScriptArg(new U32(3)) instanceof ScriptTransactionArgumentU32).toBe(true);
    expect(deserializeToScriptArg(new U64(4)) instanceof ScriptTransactionArgumentU64).toBe(true);
    expect(deserializeToScriptArg(new U128(5)) instanceof ScriptTransactionArgumentU128).toBe(true);
    expect(deserializeToScriptArg(new U256(6)) instanceof ScriptTransactionArgumentU256).toBe(true);
    expect(deserializeToScriptArg(new Bool(false)) instanceof ScriptTransactionArgumentBool).toBe(true);
    expect(
      deserializeToScriptArg(new AccountAddress(AccountAddress.FOUR)) instanceof ScriptTransactionArgumentAddress,
    ).toBe(true);
    expect(deserializeToScriptArg(MoveVector.U8([1, 2, 3, 4, 5])) instanceof ScriptTransactionArgumentU8Vector).toBe(
      true,
    );
  });
});

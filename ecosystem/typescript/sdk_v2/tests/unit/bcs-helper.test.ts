// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Bool, MoveOption, MoveString, U128, U16, U256, U32, U64, U8 } from "../../src/bcs/serializable";
import { Serializer } from "../../src/bcs/serializer";

describe("Tests for the Serializable class", () => {
  let serializer: Serializer;

  beforeEach(() => {
    serializer = new Serializer();
  });

  it("serializes and deserializes all primitive types correctly", () => {
    const u8 = new U8(1);
    const u16 = new U16(1);
    const u32 = new U32(1);
    const u64 = new U64(1);
    const u128 = new U128(1);
    const u256 = new U256(1);
    const bool = new Bool(true);
    const str = new MoveString("some string");

    u8.serialize(serializer); // or serializer.serializ(u8);
    u16.serialize(serializer); // or serializer.serialize(u16);
    u32.serialize(serializer); // or serializer.serialize(u32);
    u64.serialize(serializer); // or serializer.serialize(u64);
    u128.serialize(serializer); // or serializer.serialize((u128);
    u256.serialize(serializer); // or serializer.serialize((u256);
    bool.serialize(serializer); // or serializer.serialize((u256);
    str.serialize(serializer); // or serializer.serialize((u256);

    const u8Bytes = new Uint8Array([1]);
    const u16Bytes = new Uint8Array([1, 0]);
    const u32Bytes = new Uint8Array([1, 0, 0, 0]);
    const u64Bytes = new Uint8Array([1, 0, 0, 0, 0, 0, 0, 0]);
    const u128Bytes = new Uint8Array([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    const u256Bytes = new Uint8Array([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    const boolBytes = new Uint8Array([1]);
    const strBytes = new Uint8Array([11, 115, 111, 109, 101, 32, 115, 116, 114, 105, 110, 103]);

    // Test the toUint8Array() methods
    expect(u8.toUint8Array()).toEqual(u8Bytes);
    expect(u16.toUint8Array()).toEqual(u16Bytes);
    expect(u32.toUint8Array()).toEqual(u32Bytes);
    expect(u64.toUint8Array()).toEqual(u64Bytes);
    expect(u128.toUint8Array()).toEqual(u128Bytes);
    expect(u256.toUint8Array()).toEqual(u256Bytes);
    expect(bool.toUint8Array()).toEqual(boolBytes);
    expect(str.toUint8Array()).toEqual(strBytes);
    
    // Test the overall buffer
    expect(serializer.toUint8Array()).toEqual(new Uint8Array([...u8Bytes, ...u16Bytes, ...u32Bytes, ...u64Bytes, ...u128Bytes, ...u256Bytes, ...boolBytes, ...strBytes]));

    // Test the deserialization
    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedU8 = U8.deserialize(deserializer);
    const deserializedU16 = U16.deserialize(deserializer);
    const deserializedU32 = U32.deserialize(deserializer);
    const deserializedU64 = U64.deserialize(deserializer);
    const deserializedU128 = U128.deserialize(deserializer);
    const deserializedU256 = U256.deserialize(deserializer);
    const deserializedBool = Bool.deserialize(deserializer);
    const deserializedStr = MoveString.deserialize(deserializer);

    expect(deserializedU8.value).toEqual(u8.value);
    expect(deserializedU16.value).toEqual(u16.value);
    expect(deserializedU32.value).toEqual(u32.value);
    expect(deserializedU64.value).toEqual(u64.value);
    expect(deserializedU128.value).toEqual(u128.value);
    expect(deserializedU256.value).toEqual(u256.value);
    expect(deserializedBool.value).toEqual(bool.value);
    expect(deserializedStr.value).toEqual(str.value);
  });

  it("serializes and deserializes all option types correctly", () => {
    const u8 = new U8(1);
    const u16 = new U16(2);
    const u32 = new U32(3);
    const u64 = new U64(4);
    const u128 = new U128(5);
    const u256 = new U256(6);
    const bool = new Bool(false);
    const str = new MoveString("some string");
    
    // Create Option values for each Some(...) type and serialize them
    const someOptionU8 = new MoveOption(u8);
    const someOptionU16 = new MoveOption(u16);
    const someOptionU32 = new MoveOption(u32);
    const someOptionU64 = new MoveOption(u64);
    const someOptionU128 = new MoveOption(u128);
    const someOptionU256 = new MoveOption(u256);
    const someOptionBool = new MoveOption(bool);
    const someOptionString = new MoveOption(str);

    // Create Option values for each None(...) type and serialize them
    const noneOptionU8 = new MoveOption(null);
    const noneOptionU16 = new MoveOption(null);
    const noneOptionU32 = new MoveOption(null);
    const noneOptionU64 = new MoveOption(null);
    const noneOptionU128 = new MoveOption(null);
    const noneOptionU256 = new MoveOption(null);
    const noneOptionBool = new MoveOption(null);
    const noneOptionString = new MoveOption(null);

    serializer.serialize(someOptionU8);
    serializer.serialize(someOptionU16);
    serializer.serialize(someOptionU32);
    serializer.serialize(someOptionU64);
    serializer.serialize(someOptionU128);
    serializer.serialize(someOptionU256);
    serializer.serialize(someOptionBool);
    serializer.serialize(someOptionString);
    
    serializer.serialize(noneOptionU8);
    serializer.serialize(noneOptionU16);
    serializer.serialize(noneOptionU32);
    serializer.serialize(noneOptionU64);
    serializer.serialize(noneOptionU128);
    serializer.serialize(noneOptionU256);
    serializer.serialize(noneOptionBool);
    serializer.serialize(noneOptionString);

    const someOptionU8Bytes = new Uint8Array([1, 1]);
    const someOptionU16Bytes = new Uint8Array([1, 2, 0]);
    const someOptionU32Bytes = new Uint8Array([1, 3, 0, 0, 0]);
    const someOptionU64Bytes = new Uint8Array([1, 4, 0, 0, 0, 0, 0, 0, 0]);
    const someOptionU128Bytes = new Uint8Array([1, 5, 0, 0, 0, 0, 0, 0, 0, 0]);
    const someOptionU256Bytes = new Uint8Array([1, 6, 0, 0, 0, 0, 0, 0, 0, 0]);
    const someOptionBoolBytes = new Uint8Array([1, 0]);
    const someOptionStringBytes = new Uint8Array([1, 11, 115, 111, 109, 101, 32, 115, 116, 114, 105, 110, 103]);

    const noneOptionU8Bytes = new Uint8Array([0]);
    const noneOptionU16Bytes = new Uint8Array([0]);
    const noneOptionU32Bytes = new Uint8Array([0]);
    const noneOptionU64Bytes = new Uint8Array([0]);
    const noneOptionU128Bytes = new Uint8Array([0]);
    const noneOptionU256Bytes = new Uint8Array([0]);
    const noneOptionBoolBytes = new Uint8Array([0]);
    const noneOptionStringBytes = new Uint8Array([0]);

    // Test the toUint8Array() methods
    expect(someOptionU8.toUint8Array()).toEqual(someOptionU8Bytes);
    expect(someOptionU16.toUint8Array()).toEqual(someOptionU16Bytes);
    expect(someOptionU32.toUint8Array()).toEqual(someOptionU32Bytes);
    expect(someOptionU64.toUint8Array()).toEqual(someOptionU64Bytes);
    expect(someOptionU128.toUint8Array()).toEqual(someOptionU128Bytes);
    expect(someOptionU256.toUint8Array()).toEqual(someOptionU256Bytes);
    expect(someOptionBool.toUint8Array()).toEqual(someOptionBoolBytes);
    expect(someOptionString.toUint8Array()).toEqual(someOptionStringBytes);

    expect(noneOptionU8.toUint8Array()).toEqual(noneOptionU8Bytes);
    expect(noneOptionU16.toUint8Array()).toEqual(noneOptionU16Bytes);
    expect(noneOptionU32.toUint8Array()).toEqual(noneOptionU32Bytes);
    expect(noneOptionU64.toUint8Array()).toEqual(noneOptionU64Bytes);
    expect(noneOptionU128.toUint8Array()).toEqual(noneOptionU128Bytes);
    expect(noneOptionU256.toUint8Array()).toEqual(noneOptionU256Bytes);
    expect(noneOptionBool.toUint8Array()).toEqual(noneOptionBoolBytes);
    expect(noneOptionString.toUint8Array()).toEqual(noneOptionStringBytes);

    // Test the overall buffer
    expect(serializer.toUint8Array()).toEqual(new Uint8Array([...someOptionU8Bytes, ...someOptionU16Bytes, ...someOptionU32Bytes, ...someOptionU64Bytes, ...someOptionU128Bytes, ...someOptionU256Bytes, ...someOptionBoolBytes, ...someOptionStringBytes, ...noneOptionU8Bytes, ...noneOptionU16Bytes, ...noneOptionU32Bytes, ...noneOptionU64Bytes, ...noneOptionU128Bytes, ...noneOptionU256Bytes, ...noneOptionBoolBytes, ...noneOptionStringBytes]));

    // Test the deserialization
    const deserializer = new Deserializer(serializer.toUint8Array());
    const asdf = MoveOption.deserialize<Bool>(deserializer);
    const deserializedSomeOptionU8 = MoveOption.deserialize(deserializer, U8);
    const deserializedSomeOptionU16 = MoveOption.deserialize(deserializer, U16);
    const deserializedSomeOptionU32 = MoveOption.deserialize(deserializer, U32);
    const deserializedSomeOptionU64 = MoveOption.deserialize(deserializer, U64);
    const deserializedSomeOptionU128 = MoveOption.deserialize(deserializer, U128);
    const deserializedSomeOptionU256 = MoveOption.deserialize(deserializer, U256);
    const deserializedSomeOptionBool = MoveOption.deserialize(deserializer, Bool);
    const deserializedSomeOptionString = MoveOption.deserialize(deserializer, MoveString);


  });


});

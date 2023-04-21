// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AccountAddress } from "../../aptos_types";
import { Uint32 } from "../../bcs";
import { Deserializer } from "../../bcs/deserializer";
import {
  bcsSerializeBool,
  bcsSerializeBytes,
  bcsSerializeFixedBytes,
  bcsSerializeStr,
  bcsSerializeU128,
  bcsSerializeU16,
  bcsSerializeU32,
  bcsSerializeU8,
  bcsSerializeUint64,
  bcsToBytes,
  deserializeVector,
  serialization2DVectorWithFunc,
  serializeVector,
  serializeVectorWithFunc,
} from "../../bcs/helper";
import { Serializer } from "../../bcs/serializer";

test("serializes and deserializes a vector of serializables", () => {
  const address0 = AccountAddress.fromHex("0x1");
  const address1 = AccountAddress.fromHex("0x2");

  const serializer = new Serializer();
  serializeVector([address0, address1], serializer);

  const addresses: AccountAddress[] = deserializeVector(new Deserializer(serializer.getBytes()), AccountAddress);

  expect(addresses[0].address).toEqual(address0.address);
  expect(addresses[1].address).toEqual(address1.address);
});

test("bcsToBytes", () => {
  const address = AccountAddress.fromHex("0x1");
  bcsToBytes(address);

  expect(bcsToBytes(address)).toEqual(
    new Uint8Array([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
  );
});

test("bcsSerializeU8", () => {
  expect(bcsSerializeU8(255)).toEqual(new Uint8Array([0xff]));
});

test("bcsSerializeU16", () => {
  expect(bcsSerializeU16(65535)).toEqual(new Uint8Array([0xff, 0xff]));
});

test("bcsSerializeU32", () => {
  expect(bcsSerializeU32(4294967295)).toEqual(new Uint8Array([0xff, 0xff, 0xff, 0xff]));
});

test("bcsSerializeU64", () => {
  expect(bcsSerializeUint64(BigInt("18446744073709551615"))).toEqual(
    new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
  );
});

test("bcsSerializeU128", () => {
  expect(bcsSerializeU128(BigInt("340282366920938463463374607431768211455"))).toEqual(
    new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
  );
});

test("bcsSerializeBool", () => {
  expect(bcsSerializeBool(true)).toEqual(new Uint8Array([0x01]));
});

test("bcsSerializeStr", () => {
  expect(bcsSerializeStr("çå∞≠¢õß∂ƒ∫")).toEqual(
    new Uint8Array([
      24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e, 0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88,
      0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab,
    ]),
  );
});

test("bcsSerializeBytes", () => {
  expect(bcsSerializeBytes(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]))).toEqual(
    new Uint8Array([5, 0x41, 0x70, 0x74, 0x6f, 0x73]),
  );
});

test("bcsSerializeFixedBytes", () => {
  expect(bcsSerializeFixedBytes(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]))).toEqual(
    new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]),
  );
});

test("serializeVectorWithFunc", () => {
  expect(serializeVectorWithFunc([false, true], "serializeBool")).toEqual(new Uint8Array([0x2, 0x0, 0x1]));
});

test("serialization2DVectorWithFunc", () => {
  // Test with a 2D boolean array
  const boolArray = [
    [false, true],
    [true, false, true]
  ];
  expect(serialization2DVectorWithFunc(boolArray, "serializeBool"))
    .toEqual(new Uint8Array([0x2, 0x2, 0x0, 0x1, 0x3, 0x1, 0x0, 0x1]));
  
  // Test with a 2D integer array
  const intArray: Uint32[][] = [
    [1, 2, 3],
    [4, 5]
  ];
  expect(serialization2DVectorWithFunc(intArray, "serializeU32AsUleb128"))
    .toEqual(new Uint8Array([0x2, 0x3, 0x1, 0x2, 0x3, 0x2, 0x4, 0x5]));
  
  // Test with a 2D string array
  const stringArray = [
    ["hello", "world"],
    ["foo", "bar", "baz"]
  ];
  expect(serialization2DVectorWithFunc(stringArray, "serializeStr"))
    .toEqual(new Uint8Array([0x2, 0x2, 0x5, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x5, 0x77, 0x6f, 0x72, 
      0x6c, 0x64, 0x3, 0x3, 0x66, 0x6f, 0x6f, 0x3, 0x62, 0x61, 0x72, 0x3, 0x62, 0x61, 0x7a]));
  
  // Test with an empty 2D array
  const emptyArray: any[][] = [];
  expect(serialization2DVectorWithFunc(emptyArray, "serializeBool"))
    .toEqual(new Uint8Array([0x0]));
});

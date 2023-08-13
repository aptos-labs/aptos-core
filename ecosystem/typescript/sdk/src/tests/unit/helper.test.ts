// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import {
  AccountAddress,
  StructTag,
  TypeTag,
  TypeTagAddress,
  TypeTagBool,
  TypeTagStruct,
  TypeTagU16,
  TypeTagU256,
  TypeTagU64,
  TypeTagU8,
  objectStructTag,
  optionStructTag,
  stringStructTag,
} from "../../aptos_types";
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
  serializeVector,
  serializeVectorWithFunc,
  serializeVectorWithDepth,
} from "../../bcs/helper";
import { Serializer } from "../../bcs/serializer";
import { TxnBuilderTypes } from "../../transaction_builder";
import { HexString } from "../../utils";

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

test("serializeVectorWithDepth", () => {
  const vec = [
    [[[1, 2, 3]]],
    [[[4, 5, 6]]],
    [[[7, 8, 9]]],
    [
      [
        [1, 2, 3],
        [4, 5, 6],
      ],
    ],
    [
      [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9],
      ],
    ],
    [
      [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9],
      ],
    ],
    [
      [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9],
      ],
    ],
    [
      [
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9],
        [1, 2, 3],
        [4, 5, 6],
        [123, 234, 255],
        [1, 2, 3],
        [4, 5, 6],
        [7, 8, 9],
        [1, 2, 3],
        [4, 5, 6],
        [111, 222, 255],
      ],
    ],
  ];
  const vecAddresses = vec.map((v1) => v1.map((v2) => v2.map((v3) => v3.map((v4) => new HexString(String(v4))))));

  const stringTypeTagStruct = new TypeTagStruct(stringStructTag);
  const vecStrings = vec.map((v1) => v1.map((v2) => v2.map((v3) => v3.map((v4) => String(v4)))));

  const optionU16TypeTagStruct = new TypeTagStruct(optionStructTag(new TypeTagU16()));

  const objAddress = TxnBuilderTypes.AccountAddress.fromHex(
    "0xe46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a",
  );
  const vecObjects = vec.map((v1) =>
    v1.map((v2) => v2.map((v3) => v3.map((v4) => objAddress.toHexString().toString()))),
  );
  const objectU16TypeTagStruct = new TypeTagStruct(objectStructTag(new TypeTagU16()));

  // The following hex is generated from bcs::to_bytes in a Move contract with the above values (converted to the corresponding types)
  // You can deserialize these in Move with: from_bcs::from_bytes<vector<vector<vector<vector<T>>>>>(x"...hex_string_here...");
  const bcsNestedU8 = new HexString(
    "0x0801010301020301010304050601010307080901020301020303040506010303010203030405060307080901030301020303040506030708090103030102030304050603070809010c0301020303040506030708090301020303040506037beaff0301020303040506030708090301020303040506036fdeff",
  );
  const bcsNestedU16 = new HexString(
    "0x0801010301000200030001010304000500060001010307000800090001020301000200030003040005000600010303010002000300030400050006000307000800090001030301000200030003040005000600030700080009000103030100020003000304000500060003070008000900010c0301000200030003040005000600030700080009000301000200030003040005000600037b00ea00ff000301000200030003040005000600030700080009000301000200030003040005000600036f00de00ff00",
  );
  const bcsNestedU64 = new HexString(
    "0x0801010301000000000000000200000000000000030000000000000001010304000000000000000500000000000000060000000000000001010307000000000000000800000000000000090000000000000001020301000000000000000200000000000000030000000000000003040000000000000005000000000000000600000000000000010303010000000000000002000000000000000300000000000000030400000000000000050000000000000006000000000000000307000000000000000800000000000000090000000000000001030301000000000000000200000000000000030000000000000003040000000000000005000000000000000600000000000000030700000000000000080000000000000009000000000000000103030100000000000000020000000000000003000000000000000304000000000000000500000000000000060000000000000003070000000000000008000000000000000900000000000000010c0301000000000000000200000000000000030000000000000003040000000000000005000000000000000600000000000000030700000000000000080000000000000009000000000000000301000000000000000200000000000000030000000000000003040000000000000005000000000000000600000000000000037b00000000000000ea00000000000000ff000000000000000301000000000000000200000000000000030000000000000003040000000000000005000000000000000600000000000000030700000000000000080000000000000009000000000000000301000000000000000200000000000000030000000000000003040000000000000005000000000000000600000000000000036f00000000000000de00000000000000ff00000000000000",
  );
  const bcsNestedU256 = new HexString(
    "0x0801010301000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000001010304000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000001010307000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000090000000000000000000000000000000000000000000000000000000000000001020301000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000010303010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000030400000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000307000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000090000000000000000000000000000000000000000000000000000000000000001030301000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000030700000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000009000000000000000000000000000000000000000000000000000000000000000103030100000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000304000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000003070000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000900000000000000000000000000000000000000000000000000000000000000010c0301000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000030700000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000009000000000000000000000000000000000000000000000000000000000000000301000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000037b00000000000000000000000000000000000000000000000000000000000000ea00000000000000000000000000000000000000000000000000000000000000ff000000000000000000000000000000000000000000000000000000000000000301000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000030700000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000009000000000000000000000000000000000000000000000000000000000000000301000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000003040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000036f00000000000000000000000000000000000000000000000000000000000000de00000000000000000000000000000000000000000000000000000000000000ff00000000000000000000000000000000000000000000000000000000000000",
  );
  const bcsNestedAddresses = new HexString(
    "0x0801010300000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000301010300000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000601010300000000000000000000000000000000000000000000000000000000000000070000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000901020300000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000303000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000000000000000000006010303000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003030000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000000000060300000000000000000000000000000000000000000000000000000000000000070000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000901030300000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000303000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000000000000000000006030000000000000000000000000000000000000000000000000000000000000007000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000090103030000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030300000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000603000000000000000000000000000000000000000000000000000000000000000700000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000009010c030000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030300000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000603000000000000000000000000000000000000000000000000000000000000000700000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000009030000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030300000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000603000000000000000000000000000000000000000000000000000000000000012300000000000000000000000000000000000000000000000000000000000002340000000000000000000000000000000000000000000000000000000000000255030000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030300000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000603000000000000000000000000000000000000000000000000000000000000000700000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000009030000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000030300000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000005000000000000000000000000000000000000000000000000000000000000000603000000000000000000000000000000000000000000000000000000000000011100000000000000000000000000000000000000000000000000000000000002220000000000000000000000000000000000000000000000000000000000000255",
  );
  const bcsNestedStrings = new HexString(
    "0x0801010301310132013301010301340135013601010301370138013901020301310132013303013401350136010303013101320133030134013501360301370138013901030301310132013303013401350136030137013801390103030131013201330301340135013603013701380139010c030131013201330301340135013603013701380139030131013201330301340135013603033132330332333403323535030131013201330301340135013603013701380139030131013201330301340135013603033131310332323203323535",
  );
  const bcsNestedOptionU16 = new HexString(
    "0x0801010301010001020001030001010301040001050001060001010301070001080001090001020301010001020001030003010400010500010600010303010100010200010300030104000105000106000301070001080001090001030301010001020001030003010400010500010600030107000108000109000103030101000102000103000301040001050001060003010700010800010900010c030101000102000103000301040001050001060003010700010800010900030101000102000103000301040001050001060003017b0001ea0001ff00030101000102000103000301040001050001060003010700010800010900030101000102000103000301040001050001060003016f0001de0001ff00",
  );
  const bcsNestedObjects = new HexString(
    "0x08010103e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a010103e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a010103e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a010203e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a010303e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a010303e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a010303e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a010c03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a03e46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0ae46a3c36283330c97668b5d4693766b8626420a5701c18eb64026075c3ec8a0a",
  );
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vec, new TypeTagU8())).toString() === bcsNestedU8.toString(),
  ).toBe(true);
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vec, new TypeTagU16())).toString() === bcsNestedU16.toString(),
  ).toBe(true);
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vec, new TypeTagU64())).toString() === bcsNestedU64.toString(),
  ).toBe(true);
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vec, new TypeTagU256())).toString() === bcsNestedU256.toString(),
  ).toBe(true);
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vecStrings, stringTypeTagStruct)).toString() ===
      bcsNestedStrings.toString(),
  ).toBe(true);
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vecAddresses, new TypeTagAddress())).toString() ===
      bcsNestedAddresses.toString(),
  ).toBe(true);
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vec, optionU16TypeTagStruct)).toString() ===
      bcsNestedOptionU16.toString(),
  ).toBe(true);
  expect(
    HexString.fromBuffer(serializeVectorWithDepth(vecObjects, objectU16TypeTagStruct)).toString() ===
      bcsNestedObjects.toString(),
  ).toBe(true);
});

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Serializer } from "./serializer";

describe("BCS Serializer", () => {
  let serializer: Serializer;

  beforeEach(() => {
    serializer = new Serializer();
  });

  it("serializes a non-empty string", () => {
    serializer.serializeStr("çå∞≠¢õß∂ƒ∫");
    expect(serializer.getBytes()).toEqual(
      new Uint8Array([
        24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e, 0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88,
        0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab,
      ]),
    );
  });

  it("serializes an empty string", () => {
    serializer.serializeStr("");
    expect(serializer.getBytes()).toEqual(new Uint8Array([0]));
  });

  it("serializes dynamic length bytes", () => {
    serializer.serializeBytes(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]));
    expect(serializer.getBytes()).toEqual(new Uint8Array([5, 0x41, 0x70, 0x74, 0x6f, 0x73]));
  });

  it("serializes dynamic length bytes with zero elements", () => {
    serializer.serializeBytes(new Uint8Array([]));
    expect(serializer.getBytes()).toEqual(new Uint8Array([0]));
  });

  it("serializes fixed length bytes", () => {
    serializer.serializeFixedBytes(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]));
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]));
  });

  it("serializes fixed length bytes with zero element", () => {
    serializer.serializeFixedBytes(new Uint8Array([]));
    expect(serializer.getBytes()).toEqual(new Uint8Array([]));
  });

  it("serializes a boolean value", () => {
    serializer.serializeBool(true);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x01]));

    serializer = new Serializer();
    serializer.serializeBool(false);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x00]));
  });

  it("throws when serializing a boolean value with wrong data type", () => {
    expect(() => {
      // @ts-ignore
      serializer.serializeBool(12);
    }).toThrow("Value needs to be a boolean");
  });

  it("serializes a uint8", () => {
    serializer.serializeU8(255);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xff]));
  });

  it("throws when serializing uint8 with out of range value", () => {
    expect(() => {
      serializer.serializeU8(256);
    }).toThrow("Value is out of range");

    expect(() => {
      serializer = new Serializer();
      serializer.serializeU8(-1);
    }).toThrow("Value is out of range");
  });

  it("serializes a uint16", () => {
    serializer.serializeU16(65535);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xff, 0xff]));

    serializer = new Serializer();
    serializer.serializeU16(4660);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x34, 0x12]));
  });

  it("throws when serializing uint16 with out of range value", () => {
    expect(() => {
      serializer.serializeU16(65536);
    }).toThrow("Value is out of range");

    expect(() => {
      serializer = new Serializer();
      serializer.serializeU16(-1);
    }).toThrow("Value is out of range");
  });

  it("serializes a uint32", () => {
    serializer.serializeU32(4294967295);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xff, 0xff, 0xff, 0xff]));

    serializer = new Serializer();
    serializer.serializeU32(305419896);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x78, 0x56, 0x34, 0x12]));
  });

  it("throws when serializing uint32 with out of range value", () => {
    expect(() => {
      serializer.serializeU32(4294967296);
    }).toThrow("Value is out of range");

    expect(() => {
      serializer = new Serializer();
      serializer.serializeU32(-1);
    }).toThrow("Value is out of range");
  });

  it("serializes a uint64", () => {
    serializer.serializeU64(BigInt("18446744073709551615"));
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]));

    serializer = new Serializer();
    serializer.serializeU64(BigInt("1311768467750121216"));
    expect(serializer.getBytes()).toEqual(new Uint8Array([0x00, 0xef, 0xcd, 0xab, 0x78, 0x56, 0x34, 0x12]));
  });

  it("throws when serializing uint64 with out of range value", () => {
    expect(() => {
      serializer.serializeU64(BigInt("18446744073709551616"));
    }).toThrow("Value is out of range");

    expect(() => {
      serializer = new Serializer();
      serializer.serializeU64(-1);
    }).toThrow("Value is out of range");
  });

  it("serializes a uint128", () => {
    serializer.serializeU128(BigInt("340282366920938463463374607431768211455"));
    expect(serializer.getBytes()).toEqual(
      new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
    );

    serializer = new Serializer();
    serializer.serializeU128(BigInt("1311768467750121216"));
    expect(serializer.getBytes()).toEqual(
      new Uint8Array([0x00, 0xef, 0xcd, 0xab, 0x78, 0x56, 0x34, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    );
  });

  it("throws when serializing uint128 with out of range value", () => {
    expect(() => {
      serializer.serializeU128(BigInt("340282366920938463463374607431768211456"));
    }).toThrow("Value is out of range");

    expect(() => {
      serializer = new Serializer();
      serializer.serializeU128(-1);
    }).toThrow("Value is out of range");
  });

  it("serializes a uleb128", () => {
    serializer.serializeU32AsUleb128(104543565);
    expect(serializer.getBytes()).toEqual(new Uint8Array([0xcd, 0xea, 0xec, 0x31]));
  });

  it("throws when serializing uleb128 with out of range value", () => {
    expect(() => {
      serializer.serializeU32AsUleb128(4294967296);
    }).toThrow("Value is out of range");

    expect(() => {
      serializer = new Serializer();
      serializer.serializeU32AsUleb128(-1);
    }).toThrow("Value is out of range");
  });
});

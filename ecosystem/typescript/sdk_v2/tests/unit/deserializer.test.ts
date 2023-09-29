// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Serializable, Serializer, Deserializer } from "../../src/bcs";

describe("BCS Deserializer", () => {
  it("deserializes a non-empty string", () => {
    const deserializer = new Deserializer(
      new Uint8Array([
        24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e, 0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88,
        0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab,
      ]),
    );
    expect(deserializer.deserializeStr()).toBe("çå∞≠¢õß∂ƒ∫");
  });

  it("deserializes an empty string", () => {
    const deserializer = new Deserializer(new Uint8Array([0]));
    expect(deserializer.deserializeStr()).toBe("");
  });

  it("deserializes dynamic length bytes", () => {
    const deserializer = new Deserializer(new Uint8Array([5, 0x41, 0x70, 0x74, 0x6f, 0x73]));
    expect(deserializer.deserializeBytes()).toEqual(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]));
  });

  it("deserializes dynamic length bytes with zero elements", () => {
    const deserializer = new Deserializer(new Uint8Array([0]));
    expect(deserializer.deserializeBytes()).toEqual(new Uint8Array([]));
  });

  it("deserializes fixed length bytes", () => {
    const deserializer = new Deserializer(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]));
    expect(deserializer.deserializeFixedBytes(5)).toEqual(new Uint8Array([0x41, 0x70, 0x74, 0x6f, 0x73]));
  });

  it("deserializes fixed length bytes with zero element", () => {
    const deserializer = new Deserializer(new Uint8Array([]));
    expect(deserializer.deserializeFixedBytes(0)).toEqual(new Uint8Array([]));
  });

  it("deserializes a boolean value", () => {
    let deserializer = new Deserializer(new Uint8Array([0x01]));
    expect(deserializer.deserializeBool()).toEqual(true);
    deserializer = new Deserializer(new Uint8Array([0x00]));
    expect(deserializer.deserializeBool()).toEqual(false);
  });

  it("throws when deserializing a boolean with disallowed values", () => {
    expect(() => {
      const deserializer = new Deserializer(new Uint8Array([0x12]));
      deserializer.deserializeBool();
    }).toThrow("Invalid boolean value");
  });

  it("deserializes a uint8", () => {
    const deserializer = new Deserializer(new Uint8Array([0xff]));
    expect(deserializer.deserializeU8()).toEqual(255);
  });

  it("deserializes a uint16", () => {
    let deserializer = new Deserializer(new Uint8Array([0xff, 0xff]));
    expect(deserializer.deserializeU16()).toEqual(65535);
    deserializer = new Deserializer(new Uint8Array([0x34, 0x12]));
    expect(deserializer.deserializeU16()).toEqual(4660);
  });

  it("deserializes a uint32", () => {
    let deserializer = new Deserializer(new Uint8Array([0xff, 0xff, 0xff, 0xff]));
    expect(deserializer.deserializeU32()).toEqual(4294967295);
    deserializer = new Deserializer(new Uint8Array([0x78, 0x56, 0x34, 0x12]));
    expect(deserializer.deserializeU32()).toEqual(305419896);
  });

  it("deserializes a uint64", () => {
    let deserializer = new Deserializer(new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]));
    expect(deserializer.deserializeU64()).toEqual(BigInt("18446744073709551615"));
    deserializer = new Deserializer(new Uint8Array([0x00, 0xef, 0xcd, 0xab, 0x78, 0x56, 0x34, 0x12]));
    expect(deserializer.deserializeU64()).toEqual(BigInt("1311768467750121216"));
  });

  it("deserializes a uint128", () => {
    let deserializer = new Deserializer(
      new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
    );
    expect(deserializer.deserializeU128()).toEqual(BigInt("340282366920938463463374607431768211455"));
    deserializer = new Deserializer(
      new Uint8Array([0x00, 0xef, 0xcd, 0xab, 0x78, 0x56, 0x34, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    );
    expect(deserializer.deserializeU128()).toEqual(BigInt("1311768467750121216"));
  });
  it("deserializes a uint256", () => {
    let deserializer = new Deserializer(
      new Uint8Array([
        0x31, 0x30, 0x29, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x20, 0x19, 0x18, 0x17, 0x16, 0x15, 0x14,
        0x13, 0x12, 0x11, 0x10, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x00,
      ]),
    );
    expect(deserializer.deserializeU256()).toEqual(
      BigInt("0x0001020304050607080910111213141516171819202122232425262728293031"),
    );
  });

  it("deserializes a uleb128", () => {
    let deserializer = new Deserializer(new Uint8Array([0xcd, 0xea, 0xec, 0x31]));
    expect(deserializer.deserializeUleb128AsU32()).toEqual(104543565);

    deserializer = new Deserializer(new Uint8Array([0xff, 0xff, 0xff, 0xff, 0x0f]));
    expect(deserializer.deserializeUleb128AsU32()).toEqual(4294967295);
  });

  it("throws when deserializing a uleb128 with out ranged value", () => {
    expect(() => {
      const deserializer = new Deserializer(new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x10]));
      deserializer.deserializeUleb128AsU32();
    }).toThrow("Overflow while parsing uleb128-encoded uint32 value");
  });

  it("throws when deserializing against buffer that has been drained", () => {
    expect(() => {
      const deserializer = new Deserializer(
        new Uint8Array([
          24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e, 0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2,
          0x88, 0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab,
        ]),
      );

      deserializer.deserializeStr();
      deserializer.deserializeStr();
    }).toThrow("Reached to the end of buffer");
  });

  it("deserializes a single deserializable class", () => {
    // Define the MoveStruct class that implements the Deserializable interface
    class MoveStruct extends Serializable {
      constructor(
        public name: string,
        public description: string,
        public enabled: boolean,
        public vectorU8: Array<number>,
      ) {
        super();
      }

      serialize(serializer: Serializer): void {
        serializer.serializeStr(this.name);
        serializer.serializeStr(this.description);
        serializer.serializeBool(this.enabled);
        serializer.serializeU32AsUleb128(this.vectorU8.length);
        this.vectorU8.forEach((n) => serializer.serializeU8(n));
      }

      static deserialize(deserializer: Deserializer): MoveStruct {
        const name = deserializer.deserializeStr();
        const description = deserializer.deserializeStr();
        const enabled = deserializer.deserializeBool();
        const length = deserializer.deserializeUleb128AsU32();
        const vectorU8 = new Array<number>();
        for (let i = 0; i < length; i++) {
          vectorU8.push(deserializer.deserializeU8());
        }
        return new MoveStruct(name, description, enabled, vectorU8);
      }
    }
    // Construct a MoveStruct
    const moveStruct = new MoveStruct("abc", "123", false, [1, 2, 3, 4]);
    // Serialize a MoveStruct instance.
    const serializer = new Serializer();
    serializer.serialize(moveStruct);
    const moveStructBcsBytes = serializer.toUint8Array();
    // Load the bytes into the Deserializer buffer
    const deserializer = new Deserializer(moveStructBcsBytes);
    // Deserialize the buffered bytes into an instance of MoveStruct
    const deserializedMoveStruct = deserializer.deserialize(MoveStruct);
    expect(deserializedMoveStruct.name).toEqual(moveStruct.name);
    expect(deserializedMoveStruct.description).toEqual(moveStruct.description);
    expect(deserializedMoveStruct.enabled).toEqual(moveStruct.enabled);
    expect(deserializedMoveStruct.vectorU8).toEqual(moveStruct.vectorU8);
  });

  it("deserializes and composes an abstract Deserializable class instance from composed deserialize calls", () => {
    abstract class MoveStruct extends Serializable {
      abstract serialize(serializer: Serializer): void;

      static deserialize(deserializer: Deserializer): MoveStruct {
        const index = deserializer.deserializeUleb128AsU32();
        switch (index) {
          case 0:
            return MoveStructA.load(deserializer);
          case 1:
            return MoveStructB.load(deserializer);
          default:
            throw new Error("Invalid variant index");
        }
      }
    }

    class MoveStructA extends Serializable {
      constructor(
        public name: string,
        public description: string,
        public enabled: boolean,
        public vectorU8: Array<number>,
      ) {
        super();
      }

      serialize(serializer: Serializer): void {
        // enum variant index for the abstract MoveStruct class
        serializer.serializeU32AsUleb128(0);
        serializer.serializeStr(this.name);
        serializer.serializeStr(this.description);
        serializer.serializeBool(this.enabled);
        serializer.serializeU32AsUleb128(this.vectorU8.length);
        this.vectorU8.forEach((n) => serializer.serializeU8(n));
      }

      static load(deserializer: Deserializer): MoveStructA {
        const name = deserializer.deserializeStr();
        const description = deserializer.deserializeStr();
        const enabled = deserializer.deserializeBool();
        const length = deserializer.deserializeUleb128AsU32();
        const vectorU8 = new Array<number>();
        for (let i = 0; i < length; i++) {
          vectorU8.push(deserializer.deserializeU8());
        }
        return new MoveStructA(name, description, enabled, vectorU8);
      }
    }
    class MoveStructB extends Serializable {
      constructor(
        public moveStructA: MoveStructA,
        public name: string,
        public description: string,
        public vectorU8: Array<number>,
      ) {
        super();
      }

      serialize(serializer: Serializer): void {
        // enum variant index for the abstract MoveStruct class
        serializer.serializeU32AsUleb128(1);
        serializer.serialize(this.moveStructA);
        serializer.serializeStr(this.name);
        serializer.serializeStr(this.description);
        serializer.serializeU32AsUleb128(this.vectorU8.length);
        this.vectorU8.forEach((n) => serializer.serializeU8(n));
      }

      static load(deserializer: Deserializer): MoveStructB {
        // note we cannot use MoveStructA.load here because we need to pop off the variant index first
        const moveStructA = MoveStruct.deserialize(deserializer) as MoveStructA;
        const name = deserializer.deserializeStr();
        const description = deserializer.deserializeStr();
        const length = deserializer.deserializeUleb128AsU32();
        const vectorU8 = new Array<number>();
        for (let i = 0; i < length; i++) {
          vectorU8.push(deserializer.deserializeU8());
        }
        return new MoveStructB(moveStructA, name, description, vectorU8);
      }
    }

    // in a real e2e flow, we might get a stream of BCS-serialized bytes that we deserialize,
    // say as a wallet in a dapp, we need to deserialize the payload and read its inner fields.
    // The payload could be of multiple types, so we need to first deserialize the variant index
    // and then deserialize the payload based on the variant index.
    //
    // The abstract MoveStruct class is used to demonstrate this process.

    // Construct a MoveStructA and a MoveStructB, which consists of a MoveStructA inside it
    const moveStructA = new MoveStructA("abc", "123", false, [1, 2, 3, 4]);
    const moveStructAInsideB = new MoveStructA("def", "456", true, [5, 6, 7, 8]);
    const moveStructB = new MoveStructB(moveStructAInsideB, "ghi", "789", [9, 10, 11, 12]);

    // say for some reason we serialize two MoveStructs into a single byte array
    // and we want to deserialize them back into two MoveStruct instances later
    const serializer = new Serializer();
    serializer.serialize(moveStructA);
    serializer.serialize(moveStructB);
    const serializedBytes = serializer.toUint8Array();

    // We receive the serializedBytes somewhere else, and
    // load the bytes into the Deserializer buffer
    const deserializer = new Deserializer(serializedBytes);
    // we extract each one, and typecast them because we are expecting MoveStructA and then MoveStructB
    const deserializedMoveStructA = deserializer.deserialize(MoveStruct) as MoveStructA;
    const deserializedMoveStructB = deserializer.deserialize(MoveStruct) as MoveStructB;

    // This is the MoveStructA by itself
    expect(deserializedMoveStructA.name).toEqual("abc");
    expect(deserializedMoveStructA.description).toEqual("123");
    expect(deserializedMoveStructA.enabled).toEqual(false);
    expect(deserializedMoveStructA.vectorU8).toEqual([1, 2, 3, 4]);

    // This is the MoveStructB by itself
    // Which consists of a MoveStructA and some other fields
    expect(deserializedMoveStructB.moveStructA.name).toEqual("def");
    expect(deserializedMoveStructB.moveStructA.description).toEqual("456");
    expect(deserializedMoveStructB.moveStructA.enabled).toEqual(true);
    expect(deserializedMoveStructB.moveStructA.vectorU8).toEqual([5, 6, 7, 8]);

    expect(deserializedMoveStructB.name).toEqual("ghi");
    expect(deserializedMoveStructB.description).toEqual("789");
    expect(deserializedMoveStructB.vectorU8).toEqual([9, 10, 11, 12]);
  });
});

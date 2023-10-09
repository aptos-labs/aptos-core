// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializable, Deserializer } from "../../src/bcs/deserializer";
import { FixedBytes } from "../../src/bcs/serializable/fixed-bytes";
import { Bool, U128, U16, U256, U32, U64, U8 } from "../../src/bcs/serializable/move-primitives";
import { MoveObject, MoveOption, MoveString, MoveVector } from "../../src/bcs/serializable/move-structs";
import { Serializable, Serializer } from "../../src/bcs/serializer";
import { AccountAddress } from "../../src/core";

describe("Tests for the Serializable class", () => {
  let serializer: Serializer;

  beforeEach(() => {
    serializer = new Serializer();
  });

  it("serializes the same way with all methods of serialization", () => {
    const values = [
      new U8(1),
      new U16(1),
      new U32(1),
      new U64(1),
      new U128(1),
      new U256(1),
      new Bool(true),
      new MoveString("some string"),
    ];

    let bytes = new Uint8Array();
    const serializer2 = new Serializer();
    values.forEach((value) => {
      value.serialize(serializer);
      serializer2.serialize(value);
      bytes = new Uint8Array([...bytes, ...value.bcsToBytes()]);
    });
    expect(serializer.toUint8Array()).toEqual(serializer2.toUint8Array());
    expect(serializer.toUint8Array()).toEqual(bytes);
  });

  it("serializes all simple types correctly", () => {
    const values = [
      new U8(1),
      new U16(1),
      new U32(1),
      new U64(1),
      new U128(1),
      new U256(1),
      new Bool(true),
      new MoveString("some string"),
    ];

    const serializedValues = [
      new Uint8Array([1]),
      new Uint8Array([1, 0]),
      new Uint8Array([1, 0, 0, 0]),
      new Uint8Array([1, 0, 0, 0, 0, 0, 0, 0]),
      new Uint8Array([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
      new Uint8Array([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
      new Uint8Array([1]),
      new Uint8Array([11, 115, 111, 109, 101, 32, 115, 116, 114, 105, 110, 103]),
    ];

    let serializedBytes = new Uint8Array();
    values.forEach((_, i) => {
      const value = values[i];
      const bytes = serializedValues[i];
      serializer.serialize(value);
      expect(value.bcsToBytes()).toEqual(bytes);

      // for the overall buffer
      serializedBytes = new Uint8Array([...serializedBytes, ...value.bcsToBytes()]);
    });

    expect(serializer.toUint8Array()).toEqual(serializedBytes);
  });

  it("deserializes simple Serializable values correctly", () => {
    const values = [
      new U8(1),
      new U16(1),
      new U32(1),
      new U64(1),
      new U128(1),
      new U256(1),
      new Bool(true),
      new MoveString("some string"),
    ];
    const types = [U8, U16, U32, U64, U128, U256, Bool, MoveString];

    values.forEach((_, i) => {
      const value = values[i];
      const type = types[i];
      serializer.serialize(value);
      const deserializer = new Deserializer(value.bcsToBytes());
      const deserializedValue = type.deserialize(deserializer);
      expect(deserializedValue.value).toEqual(value.value);
    });
  });

  it("serializes and deserializes a MoveObject correctly", () => {
    const moveObject = new MoveObject(AccountAddress.ONE);
    const moveObjectBytes = AccountAddress.ONE.data;
    expect(moveObject.bcsToBytes()).toEqual(moveObjectBytes);
    const deserializer = new Deserializer(moveObjectBytes);
    const deserializedMoveObject = MoveObject.deserialize(deserializer);
    expect(deserializedMoveObject.value.equals(moveObject.value)).toEqual(true);
  });

  it("serializes and deserializes MoveOption types with defined inner values correctly", () => {
    const values = [
      new U8(1),
      new U16(2),
      new U32(3),
      new U64(4),
      new U128(5),
      new U256(6),
      new Bool(false),
      new MoveString("some string"),
    ];

    const someOptionValues = values.map((value) => new MoveOption(value));
    const someBytes = [
      new Uint8Array([1, 1]),
      new Uint8Array([1, 2, 0]),
      new Uint8Array([1, 3, 0, 0, 0]),
      new Uint8Array([1, 4, 0, 0, 0, 0, 0, 0, 0]),
      new Uint8Array([1, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
      new Uint8Array([
        1, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      ]),
      new Uint8Array([1, 0]),
      new Uint8Array([1, 11, 115, 111, 109, 101, 32, 115, 116, 114, 105, 110, 103]),
    ];

    // checks each serialized value individually
    someOptionValues.forEach((_, i) => {
      const value = someOptionValues[i];
      const bytes = someBytes[i];
      expect(value.bcsToBytes()).toEqual(bytes);

      // serializer for entire buffer comparison later
      serializer.serialize(value);
    });

    let buffer = new Uint8Array();
    someBytes.forEach((bytes) => {
      buffer = new Uint8Array([...buffer, ...bytes]);
    });

    expect(serializer.toUint8Array()).toEqual(new Uint8Array([...buffer]));

    const deserializationFunctions = [
      () => MoveOption.deserialize(deserializer, U8),
      () => MoveOption.deserialize(deserializer, U16),
      () => MoveOption.deserialize(deserializer, U32),
      () => MoveOption.deserialize(deserializer, U64),
      () => MoveOption.deserialize(deserializer, U128),
      () => MoveOption.deserialize(deserializer, U256),
      () => MoveOption.deserialize(deserializer, Bool),
      () => MoveOption.deserialize(deserializer, MoveString),
    ];

    const deserializer = new Deserializer(serializer.toUint8Array());

    someOptionValues.forEach((_, i) => {
      const value = someOptionValues[i];
      const deserializedValue = deserializationFunctions[i]();
      expect(deserializedValue.unwrap().value).toEqual(value.unwrap().value);
    });
  });

  it("serializes and deserializes MoveOption types with undefined inner values correctly", () => {
    const noneOptionValues = [
      MoveOption.U8(undefined),
      MoveOption.U16(undefined),
      MoveOption.U32(undefined),
      MoveOption.U64(undefined),
      MoveOption.U128(undefined),
      MoveOption.U256(undefined),
      MoveOption.Bool(undefined),
      MoveOption.String(undefined),
    ];
    const noneBytes = noneOptionValues.map((_) => new Uint8Array([0]));

    // checks each serialized value individually
    noneOptionValues.forEach((_, i) => {
      const value = noneOptionValues[i];
      const bytes = noneBytes[i];
      expect(value.bcsToBytes()).toEqual(bytes);

      // serializer for entire buffer comparison later
      serializer.serialize(value);
    });

    let buffer = new Uint8Array();
    noneBytes.forEach((bytes) => {
      buffer = new Uint8Array([...buffer, ...bytes]);
    });

    expect(serializer.toUint8Array()).toEqual(new Uint8Array([...buffer]));

    const deserializationFunctions = [
      () => MoveOption.deserialize(deserializer, U8),
      () => MoveOption.deserialize(deserializer, U16),
      () => MoveOption.deserialize(deserializer, U32),
      () => MoveOption.deserialize(deserializer, U64),
      () => MoveOption.deserialize(deserializer, U128),
      () => MoveOption.deserialize(deserializer, U256),
      () => MoveOption.deserialize(deserializer, Bool),
      () => MoveOption.deserialize(deserializer, MoveString),
    ];

    const deserializer = new Deserializer(serializer.toUint8Array());

    noneOptionValues.forEach((_, i) => {
      const value = noneOptionValues[i];
      const deserializedValue = deserializationFunctions[i]();
      expect(deserializedValue.isSome()).toEqual(value.isSome());
    });
  });

  it("throws an error when trying to unwrap an option with no value, before and after serialization", () => {
    function testSerdeAndUnwrap<T extends Serializable>(
      optionConstructor: () => MoveOption<T>,
      deserializationClass: Deserializable<T>,
    ) {
      const option = optionConstructor();
      expect(() => option.unwrap()).toThrow();
      serializer.serialize(option);
      const deserializer = new Deserializer(serializer.toUint8Array());
      const deserializedOption = MoveOption.deserialize(deserializer, deserializationClass);
      expect(() => deserializedOption.unwrap()).toThrow();
    }

    testSerdeAndUnwrap(MoveOption.U8, U8);
    testSerdeAndUnwrap(MoveOption.U16, U16);
    testSerdeAndUnwrap(MoveOption.U32, U32);
    testSerdeAndUnwrap(MoveOption.U64, U64);
    testSerdeAndUnwrap(MoveOption.U128, U128);
    testSerdeAndUnwrap(MoveOption.U256, U256);
    testSerdeAndUnwrap(MoveOption.Bool, Bool);
    testSerdeAndUnwrap(MoveOption.String, MoveString);
  });

  it("serializes and deserializes a Vector of MoveOption types correctly", () => {
    const optionBoolVector = new MoveVector<MoveOption<Bool>>([
      new MoveOption(new Bool(true)),
      new MoveOption(new Bool(false)),
      new MoveOption(),
    ]);
    const optionBoolVectorBytes = new Uint8Array([3, 1, 1, 1, 0, 0]);
    expect(optionBoolVectorBytes).toEqual(optionBoolVector.bcsToBytes());

    const deserializer = new Deserializer(optionBoolVectorBytes);

    class VectorOptionBools {
      static deserialize(deserializer: Deserializer): MoveVector<MoveOption<Bool>> {
        const values = new Array<MoveOption<Bool>>();
        const length = deserializer.deserializeUleb128AsU32();
        for (let i = 0; i < length; i++) {
          values.push(MoveOption.deserialize(deserializer, Bool));
        }
        return new MoveVector<MoveOption<Bool>>(values);
      }
    }

    const deserializedMoveOptionBoolVector = VectorOptionBools.deserialize(deserializer);
    expect(deserializedMoveOptionBoolVector.bcsToBytes()).toEqual(optionBoolVector.bcsToBytes());
    deserializedMoveOptionBoolVector.values.forEach((option, i) => {
      if (option.isSome()) {
        expect(option.unwrap().value).toEqual(optionBoolVector.values[i].unwrap().value);
      } else {
        expect(option.isSome()).toEqual(optionBoolVector.values[i].isSome());
      }
    });
  });

  it("serializes and deserializes nested vectors and options", () => {
    const vec = new MoveVector([new MoveOption(new Bool(true)), MoveOption.Bool(false), MoveOption.Bool()]);
    // of type Vector<MoveOption<Vector<MoveOption<Bool>>>>
    // in move this would be: vector<Option<vector<Option<bool>>>>
    const vecOfVecs = new MoveVector([new MoveOption(vec), new MoveOption(vec), new MoveOption(vec)]);
    // vector<Option<vector<Option<Bool>>>>
    // 3 Options
    //    1 vector
    //      3 options [ Option<Bool> = true, Option<Bool> = false, Option<Bool> = undefined ]
    //                                  1 1                  1 0                      0
    const optionVectorOptionBool_1_Bytes = new Uint8Array([1, 3, 1, 1, 1, 0, 0]);
    //    1 vector
    //      3 options [ Option<Bool> = true, Option<Bool> = false, Option<Bool> = undefined ]
    //                                  1 1                  1 0                      0
    const optionVectorOptionBool_2_Bytes = new Uint8Array([1, 3, 1, 1, 1, 0, 0]);
    //    1 vector
    //      3 options [ Option<Bool> = true, Option<Bool> = false, Option<Bool> = undefined ]
    //                                  1 1                  1 0                      0
    const optionVectorOptionBool_3_Bytes = new Uint8Array([1, 3, 1, 1, 1, 0, 0]);
    const vecOfVecsBytes = new Uint8Array([
      3,
      ...optionVectorOptionBool_1_Bytes,
      ...optionVectorOptionBool_2_Bytes,
      ...optionVectorOptionBool_3_Bytes,
    ]);
    expect(vecOfVecsBytes).toEqual(vecOfVecs.bcsToBytes());

    const deserializer = new Deserializer(vecOfVecsBytes);
    const deserializer2 = new Deserializer(vecOfVecsBytes);

    class VectorOptionBools {
      static deserialize(deserializer: Deserializer): MoveVector<MoveOption<Bool>> {
        const values = new Array<MoveOption<Bool>>();
        const length = deserializer.deserializeUleb128AsU32();
        for (let i = 0; i < length; i++) {
          values.push(MoveOption.deserialize(deserializer, Bool));
        }
        return new MoveVector<MoveOption<Bool>>(values);
      }
    }
    class VectorOptionVectorOptionBools {
      static deserialize(deserializer: Deserializer): MoveVector<MoveOption<MoveVector<MoveOption<Bool>>>> {
        const values = new Array<MoveOption<MoveVector<MoveOption<Bool>>>>();
        const length = deserializer.deserializeUleb128AsU32();
        for (let i = 0; i < length; i++) {
          values.push(MoveOption.deserialize(deserializer, VectorOptionBools));
        }
        return new MoveVector<MoveOption<MoveVector<MoveOption<Bool>>>>(values);
      }
    }
    class OptionVectorOptionBools {
      static deserialize(deserializer: Deserializer): MoveOption<MoveVector<MoveOption<Bool>>> {
        return MoveOption.deserialize(deserializer, VectorOptionBools);
      }
    }

    const deserializedOptionVectorOptionBoolVector = VectorOptionVectorOptionBools.deserialize(deserializer);
    const deserializedOptionVectorOptionBoolVector2 = MoveVector.deserialize(deserializer2, OptionVectorOptionBools);
    expect(deserializedOptionVectorOptionBoolVector.bcsToBytes()).toEqual(
      deserializedOptionVectorOptionBoolVector2.bcsToBytes(),
    );
  });

  it("serializes all vector types with factory methods correctly", () => {
    const boolVectorFrom = MoveVector.Bool([true, false, true]);
    const u8VectorFrom = MoveVector.U8([1, 2, 3]);
    const u16VectorFrom = MoveVector.U16([1, 2, 3]);
    const u32VectorFrom = MoveVector.U32([1, 2, 3]);
    const u64VectorFrom = MoveVector.U64([1, 2, 3]);
    const u128VectorFrom = MoveVector.U128([1, 2, 3]);
    const u256VectorFrom = MoveVector.U256([1, 2, 3]);
    const stringVectorFrom = MoveVector.String(["abc", "def", "ghi"]);

    const boolVectorBytes = new Uint8Array([3, 1, 0, 1]);
    const u8VectorBytes = new Uint8Array([3, 1, 2, 3]);
    const u16VectorBytes = new Uint8Array([3, 1, 0, 2, 0, 3, 0]);
    const u32VectorBytes = new Uint8Array([3, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
    const u64VectorBytes = new Uint8Array([3, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0]);
    const u128VectorBytes = new Uint8Array([
      3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    const u256VectorBytes = new Uint8Array([
      3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    const stringVectorBytes = new Uint8Array([3, 3, 97, 98, 99, 3, 100, 101, 102, 3, 103, 104, 105]);

    expect(boolVectorFrom.bcsToBytes()).toEqual(boolVectorBytes);
    expect(u8VectorFrom.bcsToBytes()).toEqual(u8VectorBytes);
    expect(u16VectorFrom.bcsToBytes()).toEqual(u16VectorBytes);
    expect(u32VectorFrom.bcsToBytes()).toEqual(u32VectorBytes);
    expect(u64VectorFrom.bcsToBytes()).toEqual(u64VectorBytes);
    expect(u128VectorFrom.bcsToBytes()).toEqual(u128VectorBytes);
    expect(u256VectorFrom.bcsToBytes()).toEqual(u256VectorBytes);
    expect(stringVectorFrom.bcsToBytes()).toEqual(stringVectorBytes);
  });

  it("serializes all manually constructed vector types the same way as the equivalent factory methods", () => {
    const boolVector = new MoveVector([new Bool(true), new Bool(false), new Bool(true)]);
    const boolVectorFrom = MoveVector.Bool([true, false, true]);
    const u8Vector = new MoveVector([new U8(1), new U8(2), new U8(3)]);
    const u8VectorFrom = MoveVector.U8([1, 2, 3]);
    const u16Vector = new MoveVector([new U16(1), new U16(2), new U16(3)]);
    const u16VectorFrom = MoveVector.U16([1, 2, 3]);
    const u32Vector = new MoveVector([new U32(1), new U32(2), new U32(3)]);
    const u32VectorFrom = MoveVector.U32([1, 2, 3]);
    const u64Vector = new MoveVector([new U64(1), new U64(2), new U64(3)]);
    const u64VectorFrom = MoveVector.U64([1, 2, 3]);
    const u128Vector = new MoveVector([new U128(1), new U128(2), new U128(3)]);
    const u128VectorFrom = MoveVector.U128([1, 2, 3]);
    const u256Vector = new MoveVector([new U256(1), new U256(2), new U256(3)]);
    const u256VectorFrom = MoveVector.U256([1, 2, 3]);
    const stringVector = new MoveVector([new MoveString("abc"), new MoveString("def"), new MoveString("ghi")]);
    const stringVectorFrom = MoveVector.String(["abc", "def", "ghi"]);

    expect(boolVector.bcsToBytes()).toEqual(boolVectorFrom.bcsToBytes());
    expect(u8Vector.bcsToBytes()).toEqual(u8VectorFrom.bcsToBytes());
    expect(u16Vector.bcsToBytes()).toEqual(u16VectorFrom.bcsToBytes());
    expect(u32Vector.bcsToBytes()).toEqual(u32VectorFrom.bcsToBytes());
    expect(u64Vector.bcsToBytes()).toEqual(u64VectorFrom.bcsToBytes());
    expect(u128Vector.bcsToBytes()).toEqual(u128VectorFrom.bcsToBytes());
    expect(u256Vector.bcsToBytes()).toEqual(u256VectorFrom.bcsToBytes());
    expect(stringVector.bcsToBytes()).toEqual(stringVectorFrom.bcsToBytes());
  });

  it("serializes and deserializes a complex class correctly", () => {
    class ComplexSerializable extends Serializable {
      constructor(
        public myU8: U8,
        public myU16: U16,
        public myU32: U32,
        public myU64: U64,
        public myU128: U128,
        public myU256: U256,
        public myBool: Bool,
        public myString: MoveString,
        public myVectorBool: MoveVector<Bool>,
        public myVectorU8: MoveVector<U8>,
        public myVectorU16: MoveVector<U16>,
        public myVectorU32: MoveVector<U32>,
        public myVectorU64: MoveVector<U64>,
        public myVectorU128: MoveVector<U128>,
        public myVectorU256: MoveVector<U256>,
        public myVectorString: MoveVector<MoveString>,
        public myOptionBool: MoveOption<Bool>,
        public myOptionU64: MoveOption<U64>,
        public myOptionString: MoveOption<MoveString>,
      ) {
        super();
      }

      serialize(serializer: Serializer): void {
        serializer.serialize(this.myU8);
        serializer.serialize(this.myU16);
        serializer.serialize(this.myU32);
        serializer.serialize(this.myU64);
        serializer.serialize(this.myU128);
        serializer.serialize(this.myU256);
        serializer.serialize(this.myBool);
        serializer.serialize(this.myString);
        serializer.serialize(this.myVectorBool);
        serializer.serialize(this.myVectorU8);
        serializer.serialize(this.myVectorU16);
        serializer.serialize(this.myVectorU32);
        serializer.serialize(this.myVectorU64);
        serializer.serialize(this.myVectorU128);
        serializer.serialize(this.myVectorU256);
        serializer.serialize(this.myVectorString);
        serializer.serialize(this.myOptionBool);
        serializer.serialize(this.myOptionU64);
        serializer.serialize(this.myOptionString);
      }

      static deserialize(deserializer: Deserializer): ComplexSerializable {
        return new ComplexSerializable(
          U8.deserialize(deserializer),
          U16.deserialize(deserializer),
          U32.deserialize(deserializer),
          U64.deserialize(deserializer),
          U128.deserialize(deserializer),
          U256.deserialize(deserializer),
          Bool.deserialize(deserializer),
          MoveString.deserialize(deserializer),
          MoveVector.deserialize(deserializer, Bool),
          MoveVector.deserialize(deserializer, U8),
          MoveVector.deserialize(deserializer, U16),
          MoveVector.deserialize(deserializer, U32),
          MoveVector.deserialize(deserializer, U64),
          MoveVector.deserialize(deserializer, U128),
          MoveVector.deserialize(deserializer, U256),
          MoveVector.deserialize(deserializer, MoveString),
          MoveOption.deserialize(deserializer, Bool),
          MoveOption.deserialize(deserializer, U64),
          MoveOption.deserialize(deserializer, MoveString),
        );
      }
    }

    const complexSerializable = new ComplexSerializable(
      new U8(1),
      new U16(2),
      new U32(3),
      new U64(4),
      new U128(5),
      new U256(6),
      new Bool(true),
      new MoveString("some string"),
      MoveVector.Bool([true, false, true]),
      MoveVector.U8([1, 2, 3]),
      MoveVector.U16([1, 2, 3]),
      MoveVector.U32([1, 2, 3]),
      MoveVector.U64([1, 2, 3]),
      MoveVector.U128([1, 2, 3]),
      MoveVector.U256([1, 2, 3]),
      MoveVector.String(["abc", "def", "ghi"]),
      new MoveOption(new Bool(true)),
      new MoveOption(),
      new MoveOption(new MoveString("abc")),
    );

    serializer.serialize(complexSerializable);

    const complexSerializableBytes = new Uint8Array([
      1, 2, 0, 3, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 11, 115, 111, 109, 101, 32, 115,
      116, 114, 105, 110, 103, 3, 1, 0, 1, 3, 1, 2, 3, 3, 1, 0, 2, 0, 3, 0, 3, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 3, 1,
      0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 1, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 97, 98, 99, 3, 100, 101, 102, 3, 103, 104, 105, 1, 1, 0,
      1, 3, 97, 98, 99,
    ]);

    expect(serializer.toUint8Array()).toEqual(complexSerializableBytes);
    expect(complexSerializable.bcsToBytes()).toEqual(complexSerializableBytes);
    const deserializer = new Deserializer(complexSerializable.bcsToBytes());
    const deserializedComplexSerializable = ComplexSerializable.deserialize(deserializer);
    expect(deserializedComplexSerializable.myU8.value).toEqual(complexSerializable.myU8.value);
    expect(deserializedComplexSerializable.myU16.value).toEqual(complexSerializable.myU16.value);
    expect(deserializedComplexSerializable.myU32.value).toEqual(complexSerializable.myU32.value);
    expect(deserializedComplexSerializable.myU64.value).toEqual(complexSerializable.myU64.value);
    expect(deserializedComplexSerializable.myU128.value).toEqual(complexSerializable.myU128.value);
    expect(deserializedComplexSerializable.myU256.value).toEqual(complexSerializable.myU256.value);
    expect(deserializedComplexSerializable.myBool.value).toEqual(complexSerializable.myBool.value);
    expect(deserializedComplexSerializable.myString.value).toEqual(complexSerializable.myString.value);
    expect(deserializedComplexSerializable.myVectorBool.values).toEqual(complexSerializable.myVectorBool.values);
    expect(deserializedComplexSerializable.myVectorU8.values).toEqual(complexSerializable.myVectorU8.values);
    expect(deserializedComplexSerializable.myVectorU16.values).toEqual(complexSerializable.myVectorU16.values);
    expect(deserializedComplexSerializable.myVectorU32.values).toEqual(complexSerializable.myVectorU32.values);
    expect(deserializedComplexSerializable.myVectorU64.values).toEqual(complexSerializable.myVectorU64.values);
    expect(deserializedComplexSerializable.myVectorU128.values).toEqual(complexSerializable.myVectorU128.values);
    expect(deserializedComplexSerializable.myVectorU256.values).toEqual(complexSerializable.myVectorU256.values);
    expect(deserializedComplexSerializable.myVectorString.values).toEqual(complexSerializable.myVectorString.values);
    expect(deserializedComplexSerializable.myOptionBool.value!.value).toEqual(
      complexSerializable.myOptionBool.value!.value,
    );
    expect(deserializedComplexSerializable.myOptionU64.value).toEqual(complexSerializable.myOptionU64.value);
    expect(deserializedComplexSerializable.myOptionU64.value).toEqual(undefined);
    expect(deserializedComplexSerializable.myOptionString.value!.value).toEqual(
      complexSerializable.myOptionString.value!.value,
    );
  });

  it("serializes and deserializes a FixedBytes class correctly", () => {
    const address = AccountAddress.ONE;
    const fixedBytes = new FixedBytes(address.data);
    expect(fixedBytes.bcsToBytes()).toEqual(address.data);
    const deserializer = new Deserializer(fixedBytes.bcsToBytes());
    const deserializedFixedBytes = FixedBytes.deserialize(deserializer, AccountAddress.LENGTH);
    expect(deserializedFixedBytes.value).toEqual(address.data);
  });
});

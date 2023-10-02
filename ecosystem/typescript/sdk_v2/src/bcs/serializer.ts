// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable no-bitwise */
import {
  MAX_U128_BIG_INT,
  MAX_U16_NUMBER,
  MAX_U32_NUMBER,
  MAX_U64_BIG_INT,
  MAX_U8_NUMBER,
  MAX_U256_BIG_INT,
} from "./consts";
import { AnyNumber, Uint16, Uint32, Uint8 } from "../types";

// This class is intended to be used as a base class for all serializable types.
// It can be used to facilitate composable serialization of a complex type and
// in general to serialize a type to its BCS representation.
export abstract class Serializable {
  abstract serialize(serializer: Serializer): void;

  /**
   * Serializes a `Serializable` value to its BCS representation.
   * This function is the Typescript SDK equivalent of `bcs::to_bytes` in Move.
   * @returns the BCS representation of the Serializable instance as a byte buffer
   */
  bcsToBytes(): Uint8Array {
    const serializer = new Serializer();
    this.serialize(serializer);
    return serializer.toUint8Array();
  }
}

export class Serializer {
  private buffer: ArrayBuffer;

  private offset: number;

  // Constructs a serializer with a buffer of size `length` bytes, 64 bytes by default.
  // `length` must be greater than 0.
  constructor(length: number = 64) {
    if (length <= 0) {
      throw new Error("Length needs to be greater than 0");
    }
    this.buffer = new ArrayBuffer(length);
    this.offset = 0;
  }

  private ensureBufferWillHandleSize(bytes: number) {
    while (this.buffer.byteLength < this.offset + bytes) {
      const newBuffer = new ArrayBuffer(this.buffer.byteLength * 2);
      new Uint8Array(newBuffer).set(new Uint8Array(this.buffer));
      this.buffer = newBuffer;
    }
  }

  protected appendToBuffer(values: Uint8Array) {
    this.ensureBufferWillHandleSize(values.length);
    new Uint8Array(this.buffer, this.offset).set(values);
    this.offset += values.length;
  }

  private serializeWithFunction(
    fn: (byteOffset: number, value: number, littleEndian?: boolean) => void,
    bytesLength: number,
    value: number,
  ) {
    this.ensureBufferWillHandleSize(bytesLength);
    const dv = new DataView(this.buffer, this.offset);
    fn.apply(dv, [0, value, true]);
    this.offset += bytesLength;
  }

  /**
   * Serializes a string. UTF8 string is supported.
   *
   * The number of bytes in the string content is serialized first, as a uleb128-encoded u32 integer.
   * Then the string content is serialized as UTF8 encoded bytes.
   *
   * BCS layout for "string": string_length | string_content
   * where string_length is a u32 integer encoded as a uleb128 integer, equal to the number of bytes in string_content.
   *
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeStr("1234abcd");
   * assert(serializer.toUint8Array() === new Uint8Array([8, 49, 50, 51, 52, 97, 98, 99, 100]));
   * ```
   */
  serializeStr(value: string) {
    const textEncoder = new TextEncoder();
    this.serializeBytes(textEncoder.encode(value));
  }

  /**
   * Serializes an array of bytes.
   *
   * BCS layout for "bytes": bytes_length | bytes
   * where bytes_length is a u32 integer encoded as a uleb128 integer, equal to the length of the bytes array.
   */
  serializeBytes(value: Uint8Array) {
    this.serializeU32AsUleb128(value.length);
    this.appendToBuffer(value);
  }

  /**
   * Serializes an array of bytes with known length. Therefore length doesn't need to be
   * serialized to help deserialization.
   *
   * When deserializing, the number of bytes to deserialize needs to be passed in.
   */
  serializeFixedBytes(value: Uint8Array) {
    this.appendToBuffer(value);
  }

  /**
   * Serializes a boolean value.
   *
   * BCS layout for "boolean": One byte. "0x01" for true and "0x00" for false.
   */
  serializeBool(value: boolean) {
    if (typeof value !== "boolean") {
      throw new Error("Value needs to be a boolean");
    }
    const byteValue = value ? 1 : 0;
    this.appendToBuffer(new Uint8Array([byteValue]));
  }

  /**
   * Serializes a uint8 number.
   *
   * BCS layout for "uint8": One byte. Binary format in little-endian representation.
   */
  @checkNumberRange(0, MAX_U8_NUMBER)
  serializeU8(value: Uint8) {
    this.appendToBuffer(new Uint8Array([value]));
  }

  /**
   * Serializes a uint16 number.
   *
   * BCS layout for "uint16": Two bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeU16(4660);
   * assert(serializer.toUint8Array() === new Uint8Array([0x34, 0x12]));
   * ```
   */
  @checkNumberRange(0, MAX_U16_NUMBER)
  serializeU16(value: Uint16) {
    this.serializeWithFunction(DataView.prototype.setUint16, 2, value);
  }

  /**
   * Serializes a uint32 number.
   *
   * BCS layout for "uint32": Four bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeU32(305419896);
   * assert(serializer.toUint8Array() === new Uint8Array([0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  @checkNumberRange(0, MAX_U32_NUMBER)
  serializeU32(value: Uint32) {
    this.serializeWithFunction(DataView.prototype.setUint32, 4, value);
  }

  /**
   * Serializes a uint64 number.
   *
   * BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeU64(1311768467750121216);
   * assert(serializer.toUint8Array() === new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  @checkNumberRange(BigInt(0), MAX_U64_BIG_INT)
  serializeU64(value: AnyNumber) {
    const low = BigInt(value.toString()) & BigInt(MAX_U32_NUMBER);
    const high = BigInt(value.toString()) >> BigInt(32);

    // write little endian number
    this.serializeU32(Number(low));
    this.serializeU32(Number(high));
  }

  /**
   * Serializes a uint128 number.
   *
   * BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.
   */
  @checkNumberRange(BigInt(0), MAX_U128_BIG_INT)
  serializeU128(value: AnyNumber) {
    const low = BigInt(value.toString()) & MAX_U64_BIG_INT;
    const high = BigInt(value.toString()) >> BigInt(64);

    // write little endian number
    this.serializeU64(low);
    this.serializeU64(high);
  }

  /**
   * Serializes a uint256 number.
   *
   * BCS layout for "uint256": Sixteen bytes. Binary format in little-endian representation.
   */
  @checkNumberRange(BigInt(0), MAX_U256_BIG_INT)
  serializeU256(value: AnyNumber) {
    const low = BigInt(value.toString()) & MAX_U128_BIG_INT;
    const high = BigInt(value.toString()) >> BigInt(128);

    // write little endian number
    this.serializeU128(low);
    this.serializeU128(high);
  }

  /**
   * Serializes a uint32 number with uleb128.
   *
   * BCS uses uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
   */
  @checkNumberRange(0, MAX_U32_NUMBER)
  serializeU32AsUleb128(val: Uint32) {
    let value = val;
    const valueArray = [];
    while (value >>> 7 !== 0) {
      valueArray.push((value & 0x7f) | 0x80);
      value >>>= 7;
    }
    valueArray.push(value);
    this.appendToBuffer(new Uint8Array(valueArray));
  }

  /**
   * Returns the buffered bytes
   */
  toUint8Array(): Uint8Array {
    return new Uint8Array(this.buffer).slice(0, this.offset);
  }

  /**
   * Serializes a `Serializable` value, facilitating composable serialization.
   *
   * @param value The Serializable value to serialize
   *
   * @example
   * // Define the MoveStruct class that implements the Serializable interface
   * class MoveStruct extends Serializable {
   *     constructor(
   *         public creatorAddress: AccountAddress, // where AccountAddress extends Serializable
   *         public collectionName: string,
   *         public tokenName: string
   *     ) {}
   *
   *     serialize(serializer: Serializer): void {
   *         serializer.serialize(this.creatorAddress);  // Composable serialization of another Serializable object
   *         serializer.serializeStr(this.collectionName);
   *         serializer.serializeStr(this.tokenName);
   *     }
   * }
   *
   * // Construct a MoveStruct
   * const moveStruct = new MoveStruct(new AccountAddress(...), "MyCollection", "TokenA");
   *
   * // Serialize a string, a u64 number, and a MoveStruct instance.
   * const serializer = new Serializer();
   * serializer.serializeStr("ExampleString");
   * serializer.serializeU64(12345678);
   * serializer.serialize(moveStruct);
   *
   * // Get the bytes from the Serializer instance
   * const serializedBytes = serializer.toUint8Array();
   *
   * @returns the serializer instance
   */
  serialize<T extends Serializable>(value: T) {
    // NOTE: The `serialize` method called by `value` is defined in `value`'s
    // Serializable interface, not the one defined in this class.
    value.serialize(this);
  }

  serializeVector<T extends Serializable>(values: Array<T>) {
    this.serializeU32AsUleb128(values.length);
    values.forEach((item) => {
      item.serialize(this);
    });
  }
}

/**
 * A decorator that ensures the input argument for a function is within a range.
 * @param minValue The input argument must be >= minValue
 * @param maxValue The input argument must be <= maxValue
 * @param message Error message
 */
function checkNumberRange<T extends AnyNumber>(minValue: T, maxValue: T, message?: string) {
  return (target: unknown, propertyKey: string, descriptor: PropertyDescriptor) => {
    const childFunction = descriptor.value;
    // eslint-disable-next-line no-param-reassign
    descriptor.value = function deco(value: AnyNumber): Serializer {
      const valueBigInt = BigInt(value.toString());
      if (valueBigInt > BigInt(maxValue.toString()) || valueBigInt < BigInt(minValue.toString())) {
        throw new Error(message || "Value is out of range");
      }
      return childFunction.apply(this, [value]);
    };
    return descriptor;
  };
}

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
import { AnyNumber, Uint16, Uint32, Uint8 } from "./types";

export interface Serializable {
  serialize(serializer: Serializer): void;
}

export class Serializer {
  private buffer: ArrayBuffer;

  private offset: number;

  constructor() {
    this.buffer = new ArrayBuffer(64);
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
   * assert(serializer.getBytes() === new Uint8Array([8, 49, 50, 51, 52, 97, 98, 99, 100]));
   * ```
   */
  serializeStr(value: string): this {
    const textEncoder = new TextEncoder();
    this.serializeBytes(textEncoder.encode(value));
    return this;
  }

  /**
   * Serializes an array of bytes.
   *
   * BCS layout for "bytes": bytes_length | bytes
   * where bytes_length is a u32 integer encoded as a uleb128 integer, equal to the length of the bytes array.
   */
  serializeBytes(value: Uint8Array): this {
    this.serializeU32AsUleb128(value.length);
    this.appendToBuffer(value);
    return this;
  }

  /**
   * Serializes an array of bytes with known length. Therefore length doesn't need to be
   * serialized to help deserialization.
   *
   * When deserializing, the number of bytes to deserialize needs to be passed in.
   */
  serializeFixedBytes(value: Uint8Array): this {
    this.appendToBuffer(value);
    return this;
  }

  /**
   * Serializes a boolean value.
   *
   * BCS layout for "boolean": One byte. "0x01" for true and "0x00" for false.
   */
  serializeBool(value: boolean): this {
    if (typeof value !== "boolean") {
      throw new Error("Value needs to be a boolean");
    }
    const byteValue = value ? 1 : 0;
    this.appendToBuffer(new Uint8Array([byteValue]));
    return this;
  }

  /**
   * Serializes a uint8 number.
   *
   * BCS layout for "uint8": One byte. Binary format in little-endian representation.
   */
  @checkNumberRange(0, MAX_U8_NUMBER)
  serializeU8(value: Uint8): this {
    this.appendToBuffer(new Uint8Array([value]));
    return this;
  }

  /**
   * Serializes a uint16 number.
   *
   * BCS layout for "uint16": Two bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeU16(4660);
   * assert(serializer.getBytes() === new Uint8Array([0x34, 0x12]));
   * ```
   */
  @checkNumberRange(0, MAX_U16_NUMBER)
  serializeU16(value: Uint16): this {
    this.serializeWithFunction(DataView.prototype.setUint16, 2, value);
    return this;
  }

  /**
   * Serializes a uint32 number.
   *
   * BCS layout for "uint32": Four bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeU32(305419896);
   * assert(serializer.getBytes() === new Uint8Array([0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  @checkNumberRange(0, MAX_U32_NUMBER)
  serializeU32(value: Uint32): this {
    this.serializeWithFunction(DataView.prototype.setUint32, 4, value);
    return this;
  }

  /**
   * Serializes a uint64 number.
   *
   * BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeU64(1311768467750121216);
   * assert(serializer.getBytes() === new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  @checkNumberRange(BigInt(0), MAX_U64_BIG_INT)
  serializeU64(value: AnyNumber): this {
    const low = BigInt(value.toString()) & BigInt(MAX_U32_NUMBER);
    const high = BigInt(value.toString()) >> BigInt(32);

    // write little endian number
    this.serializeU32(Number(low));
    this.serializeU32(Number(high));
    return this;
  }

  /**
   * Serializes a uint128 number.
   *
   * BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.
   */
  @checkNumberRange(BigInt(0), MAX_U128_BIG_INT)
  serializeU128(value: AnyNumber): this {
    const low = BigInt(value.toString()) & MAX_U64_BIG_INT;
    const high = BigInt(value.toString()) >> BigInt(64);

    // write little endian number
    this.serializeU64(low);
    this.serializeU64(high);
    return this;
  }

  /**
   * Serializes a uint256 number.
   *
   * BCS layout for "uint256": Sixteen bytes. Binary format in little-endian representation.
   */
  @checkNumberRange(BigInt(0), MAX_U256_BIG_INT)
  serializeU256(value: AnyNumber): this {
    const low = BigInt(value.toString()) & MAX_U128_BIG_INT;
    const high = BigInt(value.toString()) >> BigInt(128);

    // write little endian number
    this.serializeU128(low);
    this.serializeU128(high);
    return this;
  }

  /**
   * Serializes a uint32 number with uleb128.
   *
   * BCS uses uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
   */
  @checkNumberRange(0, MAX_U32_NUMBER)
  serializeU32AsUleb128(val: Uint32): this {
    let value = val;
    const valueArray = [];
    while (value >>> 7 !== 0) {
      valueArray.push((value & 0x7f) | 0x80);
      value >>>= 7;
    }
    valueArray.push(value);
    this.appendToBuffer(new Uint8Array(valueArray));
    return this;
  }

  /**
   * Serializes a `Serializable` value, facilitating composable serialization.
   *
   * @param value The value to serialize
   *
   * @example
   * // Define the MoveStruct class that implements the Serializable interface
   * class MoveStruct implements Serializable {
   *     constructor(
   *         public creator_address: AccountAddress,
   *         public collection_name: string,
   *         public token_name: string
   *     ) {}
   *
   *     serialize(serializer: Serializer): void {
   *         serializer
   *             .serialize(this.creator_address)  // Composable serialization of another Serializable object
   *             .serializeStr(this.collection_name)
   *             .serializeStr(this.token_name);
   *     }
   * }
   *
   * // Serialize a string, a u64 number, and a MoveStruct.
   * const serializedData = new Serializer()
   *     .serializeStr("ExampleString")
   *     .serializeU64(12345678)
   *     .serialize(new MoveStruct(new AccountAddress(...), "MyCollection", "TokenA"))
   *     .getBytes();
   *
   * @returns the serializer instance
   */
  serialize<T extends Serializable>(value: T): this {
    // NOTE: The `serialize` method called by `value` is defined in the
    // Serializable interface, not the one defined in this class.
    value.serialize(this);
    return this;
  }

  /**
   * Returns the buffered bytes
   */
  getBytes(): Uint8Array {
    return new Uint8Array(this.buffer).slice(0, this.offset);
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

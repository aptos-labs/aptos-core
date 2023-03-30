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
import { AnyNumber, Bytes, Uint16, Uint32, Uint8 } from "./types";

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

  protected serialize(values: Bytes) {
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
   * Serializes a string. UTF8 string is supported. Serializes the string's bytes length "l" first,
   * and then serializes "l" bytes of the string content.
   *
   * BCS layout for "string": string_length | string_content. string_length is the bytes length of
   * the string that is uleb128 encoded. string_length is a u32 integer.
   *
   * @example
   * ```ts
   * const serializer = new Serializer();
   * serializer.serializeStr("çå∞≠¢õß∂ƒ∫");
   * assert(serializer.getBytes() === new Uint8Array([24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e,
   * 0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88, 0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab]));
   * ```
   */
  serializeStr(value: string): void {
    const textEncoder = new TextEncoder();
    this.serializeBytes(textEncoder.encode(value));
  }

  /**
   * Serializes an array of bytes.
   *
   * BCS layout for "bytes": bytes_length | bytes. bytes_length is the length of the bytes array that is
   * uleb128 encoded. bytes_length is a u32 integer.
   */
  serializeBytes(value: Bytes): void {
    this.serializeU32AsUleb128(value.length);
    this.serialize(value);
  }

  /**
   * Serializes an array of bytes with known length. Therefore length doesn't need to be
   * serialized to help deserialization.  When deserializing, the number of
   * bytes to deserialize needs to be passed in.
   */
  serializeFixedBytes(value: Bytes): void {
    this.serialize(value);
  }

  /**
   * Serializes a boolean value.
   *
   * BCS layout for "boolean": One byte. "0x01" for True and "0x00" for False.
   */
  serializeBool(value: boolean): void {
    if (typeof value !== "boolean") {
      throw new Error("Value needs to be a boolean");
    }
    const byteValue = value ? 1 : 0;
    this.serialize(new Uint8Array([byteValue]));
  }

  /**
   * Serializes a uint8 number.
   *
   * BCS layout for "uint8": One byte. Binary format in little-endian representation.
   */
  @checkNumberRange(0, MAX_U8_NUMBER)
  serializeU8(value: Uint8): void {
    this.serialize(new Uint8Array([value]));
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
  serializeU16(value: Uint16): void {
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
   * assert(serializer.getBytes() === new Uint8Array([0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  @checkNumberRange(0, MAX_U32_NUMBER)
  serializeU32(value: Uint32): void {
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
   * assert(serializer.getBytes() === new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  @checkNumberRange(BigInt(0), MAX_U64_BIG_INT)
  serializeU64(value: AnyNumber): void {
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
  serializeU128(value: AnyNumber): void {
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
  serializeU256(value: AnyNumber): void {
    const low = BigInt(value.toString()) & MAX_U128_BIG_INT;
    const high = BigInt(value.toString()) >> BigInt(128);

    // write little endian number
    this.serializeU128(low);
    this.serializeU128(high);
  }

  /**
   * Serializes a uint32 number with uleb128.
   *
   * BCS use uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
   */
  @checkNumberRange(0, MAX_U32_NUMBER)
  serializeU32AsUleb128(val: Uint32): void {
    let value = val;
    const valueArray = [];
    while (value >>> 7 !== 0) {
      valueArray.push((value & 0x7f) | 0x80);
      value >>>= 7;
    }
    valueArray.push(value);
    this.serialize(new Uint8Array(valueArray));
  }

  /**
   * Returns the buffered bytes
   */
  getBytes(): Bytes {
    return new Uint8Array(this.buffer).slice(0, this.offset);
  }
}

/**
 * Creates a decorator to make sure the arg value of the decorated function is within a range.
 * @param minValue The arg value of decorated function must >= minValue
 * @param maxValue The arg value of decorated function must <= maxValue
 * @param message Error message
 */
function checkNumberRange<T extends AnyNumber>(minValue: T, maxValue: T, message?: string) {
  return (target: unknown, propertyKey: string, descriptor: PropertyDescriptor) => {
    const childFunction = descriptor.value;
    // eslint-disable-next-line no-param-reassign
    descriptor.value = function deco(value: AnyNumber) {
      const valueBigInt = BigInt(value.toString());
      if (valueBigInt > BigInt(maxValue.toString()) || valueBigInt < BigInt(minValue.toString())) {
        throw new Error(message || "Value is out of range");
      }
      childFunction.apply(this, [value]);
    };
    return descriptor;
  };
}

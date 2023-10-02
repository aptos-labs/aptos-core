// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable no-bitwise */
import { MAX_U32_NUMBER } from "./consts";
import { Uint128, Uint16, Uint256, Uint32, Uint64, Uint8 } from "../types";

/**
 * This interface exists solely for the `deserialize` function in the `Deserializer` class.
 * It is not exported because exporting it results in more typing errors than it prevents
 * due to Typescript's lack of support for static methods in abstract classes and interfaces.
 */
interface Deserializable<T> {
  deserialize(deserializer: Deserializer): T;
}

export class Deserializer {
  private buffer: ArrayBuffer;

  private offset: number;

  constructor(data: Uint8Array) {
    // copies data to prevent outside mutation of buffer.
    this.buffer = new ArrayBuffer(data.length);
    new Uint8Array(this.buffer).set(data, 0);
    this.offset = 0;
  }

  private read(length: number): ArrayBuffer {
    if (this.offset + length > this.buffer.byteLength) {
      throw new Error("Reached to the end of buffer");
    }

    const bytes = this.buffer.slice(this.offset, this.offset + length);
    this.offset += length;
    return bytes;
  }

  /**
   * Deserializes a string. UTF8 string is supported. Reads the string's bytes length "l" first,
   * and then reads "l" bytes of content. Decodes the byte array into a string.
   *
   * BCS layout for "string": string_length | string_content
   * where string_length is a u32 integer encoded as a uleb128 integer, equal to the number of bytes in string_content.
   *
   * @example
   * ```ts
   * const deserializer = new Deserializer(new Uint8Array([8, 49, 50, 51, 52, 97, 98, 99, 100]));
   * assert(deserializer.deserializeStr() === "1234abcd");
   * ```
   */
  deserializeStr(): string {
    const value = this.deserializeBytes();
    const textDecoder = new TextDecoder();
    return textDecoder.decode(value);
  }

  /**
   * Deserializes an array of bytes.
   *
   * BCS layout for "bytes": bytes_length | bytes
   * where bytes_length is a u32 integer encoded as a uleb128 integer, equal to the length of the bytes array.
   */
  deserializeBytes(): Uint8Array {
    const len = this.deserializeUleb128AsU32();
    return new Uint8Array(this.read(len));
  }

  /**
   * Deserializes an array of bytes. The number of bytes to read is already known.
   *
   */
  deserializeFixedBytes(len: number): Uint8Array {
    return new Uint8Array(this.read(len));
  }

  /**
   * Deserializes a boolean value.
   *
   * BCS layout for "boolean": One byte. "0x01" for true and "0x00" for false.
   */
  deserializeBool(): boolean {
    const bool = new Uint8Array(this.read(1))[0];
    if (bool !== 1 && bool !== 0) {
      throw new Error("Invalid boolean value");
    }
    return bool === 1;
  }

  /**
   * Deserializes a uint8 number.
   *
   * BCS layout for "uint8": One byte. Binary format in little-endian representation.
   */
  deserializeU8(): Uint8 {
    return new DataView(this.read(1)).getUint8(0);
  }

  /**
   * Deserializes a uint16 number.
   *
   * BCS layout for "uint16": Two bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const deserializer = new Deserializer(new Uint8Array([0x34, 0x12]));
   * assert(deserializer.deserializeU16() === 4660);
   * ```
   */
  deserializeU16(): Uint16 {
    return new DataView(this.read(2)).getUint16(0, true);
  }

  /**
   * Deserializes a uint32 number.
   *
   * BCS layout for "uint32": Four bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const deserializer = new Deserializer(new Uint8Array([0x78, 0x56, 0x34, 0x12]));
   * assert(deserializer.deserializeU32() === 305419896);
   * ```
   */
  deserializeU32(): Uint32 {
    return new DataView(this.read(4)).getUint32(0, true);
  }

  /**
   * Deserializes a uint64 number.
   *
   * BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const deserializer = new Deserializer(new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
   * assert(deserializer.deserializeU64() === 1311768467750121216);
   * ```
   */
  deserializeU64(): Uint64 {
    const low = this.deserializeU32();
    const high = this.deserializeU32();

    // combine the two 32-bit values and return (little endian)
    return BigInt((BigInt(high) << BigInt(32)) | BigInt(low));
  }

  /**
   * Deserializes a uint128 number.
   *
   * BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.
   */
  deserializeU128(): Uint128 {
    const low = this.deserializeU64();
    const high = this.deserializeU64();

    // combine the two 64-bit values and return (little endian)
    return BigInt((high << BigInt(64)) | low);
  }

  /**
   * Deserializes a uint256 number.
   *
   * BCS layout for "uint256": Thirty-two bytes. Binary format in little-endian representation.
   */
  deserializeU256(): Uint256 {
    const low = this.deserializeU128();
    const high = this.deserializeU128();

    // combine the two 128-bit values and return (little endian)
    return BigInt((high << BigInt(128)) | low);
  }

  /**
   * Deserializes a uleb128 encoded uint32 number.
   *
   * BCS use uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
   */
  deserializeUleb128AsU32(): Uint32 {
    let value: bigint = BigInt(0);
    let shift = 0;

    while (value < MAX_U32_NUMBER) {
      const byte = this.deserializeU8();
      value |= BigInt(byte & 0x7f) << BigInt(shift);

      if ((byte & 0x80) === 0) {
        break;
      }
      shift += 7;
    }

    if (value > MAX_U32_NUMBER) {
      throw new Error("Overflow while parsing uleb128-encoded uint32 value");
    }

    return Number(value);
  }

  /**
   * Helper function that primarily exists to support alternative syntax for deserialization.
   * That is, if we have a `const deserializer: new Deserializer(...)`, instead of having to use
   * `MyClass.deserialize(deserializer)`, we can call `deserializer.deserialize(MyClass)`.
   *
   * @example const deserializer = new Deserializer(new Uint8Array([1, 2, 3]));
   * const value = deserializer.deserialize(MyClass); // where MyClass has a `deserialize` function
   * // value is now an instance of MyClass
   * // equivalent to `const value = MyClass.deserialize(deserializer)`
   * @param cls The BCS-deserializable class to deserialize the buffered bytes into.
   *
   * @returns the deserialized value of class type T
   */
  deserialize<T>(cls: Deserializable<T>): T {
    // NOTE: `deserialize` in `cls.deserialize(this)` here is a static method defined in `cls`,
    // It is separate from the `deserialize` instance method defined here in Deserializer.
    return cls.deserialize(this);
  }
}

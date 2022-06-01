/* eslint-disable no-bitwise */
import { MAX_U32_NUMBER } from "./helper";
import { bytes, uint128, uint16, uint32, uint64, uint8 } from "./types";

export class Deserializer {
  private buffer: ArrayBuffer;

  private offset: number;

  constructor(data: bytes) {
    // copies data to prevent outside mutation of buffer.
    this.buffer = new ArrayBuffer(data.length);
    new Uint8Array(this.buffer).set(data, 0);
    this.offset = 0;
  }

  private read(length: number): ArrayBuffer {
    const bytes = this.buffer.slice(this.offset, this.offset + length);
    this.offset += length;
    return bytes;
  }

  /**
   * Deserializes a string. UTF8 string is supported. Reads the string's bytes length "l" first,
   * and then reads "l" bytes of content. Decodes the byte array into a string.
   *
   * BCS layout for "string": string_length | string_content. string_length is the bytes length of
   * the string that is uleb128 encoded. string_length is a u32 integer.
   *
   * @example
   * ```ts
   * const deserializer = new Deserializer(new Uint8Array([24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e,
   * 0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88, 0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab]));
   * assert(deserializer.deserializeStr() === "çå∞≠¢õß∂ƒ∫");
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
   * BCS layout for "bytes": bytes_length | bytes. bytes_length is the length of the bytes array that is
   * uleb128 encoded. bytes_length is a u32 integer.
   */
  deserializeBytes(): bytes {
    const len = this.deserializeUleb128AsU32();
    if (len < 0) {
      throw new Error("Length of a bytes array can't be negative");
    }
    return new Uint8Array(this.read(len));
  }

  /**
   * Deserializes an array of bytes. The number of bytes to read is already known.
   *
   */
  deserializeFixedBytes(len: number): bytes {
    return new Uint8Array(this.read(len));
  }

  /**
   * Deserializes a boolean value.
   *
   * BCS layout for "boolean": One byte. "0x01" for True and "0x00" for False.
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
  deserializeU8(): uint8 {
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
  deserializeU16(): uint16 {
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
  deserializeU32(): uint32 {
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
  deserializeU64(): uint64 {
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
  deserializeU128(): uint128 {
    const low = this.deserializeU64();
    const high = this.deserializeU64();

    // combine the two 64-bit values and return (little endian)
    return BigInt((high << BigInt(64)) | low);
  }

  /**
   * Deserializes a uleb128 encoded uint32 number.
   *
   * BCS use uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
   */
  deserializeUleb128AsU32(): uint32 {
    let value = 0;
    for (let shift = 0; shift < 32; shift += 7) {
      const x = this.deserializeU8();
      const digit = x & 0x7f;
      value |= digit << shift;
      if (value < 0 || value > MAX_U32_NUMBER) {
        throw new Error("Overflow while parsing uleb128-encoded uint32 value");
      }
      if (digit === x) {
        if (shift > 0 && digit === 0) {
          throw new Error("Invalid uleb128 number (unexpected zero digit)");
        }
        return value;
      }
    }
    throw new Error("Overflow while parsing uleb128-encoded uint32 value");
  }
}

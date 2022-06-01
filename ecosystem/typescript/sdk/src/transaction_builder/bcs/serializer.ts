import { bytes } from "./types";

export interface Serializer {
  /**
   * Serializes a string. UTF8 string is supported. Serializes the string's bytes length "l" first,
   * and then serializes "l" bytes of the string content.
   *
   * BCS layout for "string": string_length | string_content. string_length is the bytes length of
   * the string that is uleb128 encoded. string_length is a u32 integer.
   *
   * @example
   * ```ts
   * const serializer = new BcsSerializer();
   * serializer.serializeStr("çå∞≠¢õß∂ƒ∫");
   * assert(serializer.getBytes() === new Uint8Array([24, 0xc3, 0xa7, 0xc3, 0xa5, 0xe2, 0x88, 0x9e,
   * 0xe2, 0x89, 0xa0, 0xc2, 0xa2, 0xc3, 0xb5, 0xc3, 0x9f, 0xe2, 0x88, 0x82, 0xc6, 0x92, 0xe2, 0x88, 0xab]));
   * ```
   */
  serializeStr(value: string): void;

  /**
   * Serializes an array of bytes.
   *
   * BCS layout for "bytes": bytes_length | bytes. bytes_length is the length of the bytes array that is
   * uleb128 encoded. bytes_length is a u32 integer.
   */
  serializeBytes(value: bytes): void;

  /**
   * Serializes an array of bytes without prefixing with the length. When deserializing, the number of
   * bytes to deserialize needs to be passed in.
   */
  serializeFixedBytes(value: bytes): void;

  /**
   * Serializes a boolean value.
   *
   * BCS layout for "boolean": One byte. "0x01" for True and "0x00" for False.
   */
  serializeBool(value: boolean): void;

  /**
   * Serializes a uint8 number.
   *
   * BCS layout for "uint8": One byte. Binary format in little-endian representation.
   */
  serializeU8(value: number): void;

  /**
   * Serializes a uint16 number.
   *
   * BCS layout for "uint16": Two bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new BcsSerializer();
   * serializer.serializeU16(4660);
   * assert(serializer.getBytes() === new Uint8Array([0x34, 0x12]));
   * ```
   */
  serializeU16(value: number): void;

  /**
   * Serializes a uint32 number.
   *
   * BCS layout for "uint32": Four bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new BcsSerializer();
   * serializer.serializeU32(305419896);
   * assert(serializer.getBytes() === new Uint8Array([0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  serializeU32(value: number): void;

  /**
   * Serializes a uint64 number.
   *
   * BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.
   * @example
   * ```ts
   * const serializer = new BcsSerializer();
   * serializer.serializeU64(1311768467750121216);
   * assert(serializer.getBytes() === new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
   * ```
   */
  serializeU64(value: bigint | number): void;

  /**
   * Serializes a uint128 number.
   *
   * BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.
   */
  serializeU128(value: bigint | number): void;

  /**
   * Serializes a uint32 number with uleb128.
   *
   * BCS use uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
   */
  serializeU32AsUleb128(value: number): void;

  /**
   * Returns the buffered bytes
   */
  getBytes(): bytes;
}

export class BcsSerializer implements Serializer {
  private static readonly BIG_MAX_U32: bigint = BigInt("4294967295");

  private static readonly BIG_MAX_U64: bigint = BigInt("18446744073709551615");

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

  protected serialize(values: Uint8Array) {
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

  serializeStr(value: string): void {
    const textEncoder = new TextEncoder();
    this.serializeBytes(textEncoder.encode(value));
  }

  serializeBytes(value: Uint8Array): void {
    this.serializeU32AsUleb128(value.length);
    this.serialize(value);
  }

  serializeFixedBytes(value: Uint8Array): void {
    this.serialize(value);
  }

  serializeBool(value: boolean): void {
    const byteValue = value ? 1 : 0;
    this.serialize(new Uint8Array([byteValue]));
  }

  serializeU8(value: number): void {
    this.serialize(new Uint8Array([value]));
  }

  serializeU16(value: number): void {
    this.serializeWithFunction(DataView.prototype.setUint16, 2, value);
  }

  serializeU32(value: number): void {
    this.serializeWithFunction(DataView.prototype.setUint32, 4, value);
  }

  serializeU64(value: BigInt | number): void {
    const low = BigInt(value.toString()) & BcsSerializer.BIG_MAX_U32;
    const high = BigInt(value.toString()) >> BigInt(32);

    // write little endian number
    this.serializeU32(Number(low));
    this.serializeU32(Number(high));
  }

  serializeU128(value: BigInt | number): void {
    const low = BigInt(value.toString()) & BcsSerializer.BIG_MAX_U64;
    const high = BigInt(value.toString()) >> BigInt(64);

    // write little endian number
    this.serializeU64(low);
    this.serializeU64(high);
  }

  getBytes(): Uint8Array {
    return new Uint8Array(this.buffer).slice(0, this.offset);
  }

  serializeU32AsUleb128(value: number): void {
    const valueArray = [];
    while (value >>> 7 !== 0) {
      valueArray.push((value & 0x7f) | 0x80);
      value >>>= 7;
    }
    valueArray.push(value);
    this.serialize(new Uint8Array(valueArray));
  }
}

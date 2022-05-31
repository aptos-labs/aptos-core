/* eslint-disable no-bitwise */
export interface Deserializer {
  deserializeStr(): string;

  deserializeBytes(): Uint8Array;

  deserializeFixedBytes(len: number): Uint8Array;

  deserializeBool(): boolean;

  deserializeU8(): number;

  deserializeU16(): number;

  deserializeU32(): number;

  deserializeU64(): bigint;

  deserializeU128(): bigint;

  deserializeLen(): number;

  deserializeVariantIndex(): number;

  deserializeOptionTag(): boolean;

  getBufferOffset(): number;
}

export class BcsDeserializer implements Deserializer {
  private static readonly NUM_MAX_U32 = 4294967295;

  private static readonly textDecoder = new TextDecoder();

  public buffer: ArrayBuffer;

  public offset: number;

  constructor(data: Uint8Array) {
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

  public deserializeStr(): string {
    const value = this.deserializeBytes();
    return BcsDeserializer.textDecoder.decode(value);
  }

  public deserializeBytes(): Uint8Array {
    const len = this.deserializeLen();
    if (len < 0) {
      throw new Error("Length of a bytes array can't be negative");
    }
    return new Uint8Array(this.read(len));
  }

  public deserializeFixedBytes(len: number): Uint8Array {
    return new Uint8Array(this.read(len));
  }

  public deserializeBool(): boolean {
    const bool = new Uint8Array(this.read(1))[0];
    return bool === 1;
  }

  public deserializeU8(): number {
    return new DataView(this.read(1)).getUint8(0);
  }

  public deserializeU16(): number {
    return new DataView(this.read(2)).getUint16(0, true);
  }

  public deserializeU32(): number {
    return new DataView(this.read(4)).getUint32(0, true);
  }

  public deserializeU64(): bigint {
    const low = this.deserializeU32();
    const high = this.deserializeU32();

    // combine the two 32-bit values and return (little endian)
    return BigInt((BigInt(high.toString()) << BigInt(32)) | BigInt(low.toString()));
  }

  public deserializeU128(): bigint {
    const low = this.deserializeU64();
    const high = this.deserializeU64();

    // combine the two 64-bit values and return (little endian)
    return BigInt((BigInt(high.toString()) << BigInt(64)) | BigInt(low.toString()));
  }

  public deserializeOptionTag(): boolean {
    return this.deserializeBool();
  }

  public getBufferOffset(): number {
    return this.offset;
  }

  public deserializeUleb128AsU32(): number {
    let value = 0;
    for (let shift = 0; shift < 32; shift += 7) {
      const x = this.deserializeU8();
      const digit = x & 0x7f;
      value |= digit << shift;
      if (value < 0 || value > BcsDeserializer.NUM_MAX_U32) {
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

  deserializeLen(): number {
    return this.deserializeUleb128AsU32();
  }

  public deserializeVariantIndex(): number {
    return this.deserializeUleb128AsU32();
  }
}

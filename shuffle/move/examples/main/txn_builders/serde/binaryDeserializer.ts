import { Deserializer } from "./deserializer.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";

export abstract class BinaryDeserializer implements Deserializer {
  private static readonly BIG_32: bigint = BigInt(32);
  private static readonly BIG_64: bigint = BigInt(64);
  private static readonly textDecoder = typeof window === "undefined"
    ? new util.TextDecoder()
    : new TextDecoder();
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

  abstract deserializeLen(): number;

  abstract deserializeVariantIndex(): number;

  abstract checkThatKeySlicesAreIncreasing(
    key1: [number, number],
    key2: [number, number],
  ): void;

  public deserializeStr(): string {
    const value = this.deserializeBytes();
    return BinaryDeserializer.textDecoder.decode(value);
  }

  public deserializeBytes(): Uint8Array {
    const len = this.deserializeLen();
    if (len < 0) {
      throw new Error("Length of a bytes array can't be negative");
    }
    return new Uint8Array(this.read(len));
  }

  public deserializeBool(): boolean {
    const bool = new Uint8Array(this.read(1))[0];
    return bool == 1;
  }

  public deserializeUnit(): null {
    return null;
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
    return BigInt(
      (BigInt(high.toString()) << BinaryDeserializer.BIG_32) |
        BigInt(low.toString()),
    );
  }

  public deserializeU128(): bigint {
    const low = this.deserializeU64();
    const high = this.deserializeU64();

    // combine the two 64-bit values and return (little endian)
    return BigInt(
      (BigInt(high.toString()) << BinaryDeserializer.BIG_64) |
        BigInt(low.toString()),
    );
  }

  public deserializeI8(): number {
    return new DataView(this.read(1)).getInt8(0);
  }

  public deserializeI16(): number {
    return new DataView(this.read(2)).getInt16(0, true);
  }

  public deserializeI32(): number {
    return new DataView(this.read(4)).getInt32(0, true);
  }

  public deserializeI64(): bigint {
    const low = this.deserializeI32();
    const high = this.deserializeI32();

    // combine the two 32-bit values and return (little endian)
    return (BigInt(high.toString()) << BinaryDeserializer.BIG_32) |
      BigInt(low.toString());
  }

  public deserializeI128(): bigint {
    const low = this.deserializeI64();
    const high = this.deserializeI64();

    // combine the two 64-bit values and return (little endian)
    return (BigInt(high.toString()) << BinaryDeserializer.BIG_64) |
      BigInt(low.toString());
  }

  public deserializeOptionTag(): boolean {
    return this.deserializeBool();
  }

  public getBufferOffset(): number {
    return this.offset;
  }

  public deserializeChar(): string {
    throw new Error("Method deserializeChar not implemented.");
  }

  public deserializeF32(): number {
    return new DataView(this.read(4)).getFloat32(0, true);
  }

  public deserializeF64(): number {
    return new DataView(this.read(8)).getFloat64(0, true);
  }
}

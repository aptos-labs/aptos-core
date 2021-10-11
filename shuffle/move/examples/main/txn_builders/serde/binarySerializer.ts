import { Serializer } from "./serializer.ts";
import * as util from "https://deno.land/std@0.85.0/node/util.ts";

export abstract class BinarySerializer implements Serializer {
  private static readonly BIG_32: bigint = BigInt(32);
  private static readonly BIG_64: bigint = BigInt(64);

  private static readonly BIG_32Fs: bigint = BigInt("4294967295");
  private static readonly BIG_64Fs: bigint = BigInt("18446744073709551615");

  private static readonly textEncoder = typeof window === "undefined"
    ? new util.TextEncoder()
    : new TextEncoder();

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

  abstract serializeLen(value: number): void;

  abstract serializeVariantIndex(value: number): void;

  abstract sortMapEntries(offsets: number[]): void;

  public serializeStr(value: string): void {
    this.serializeBytes(BinarySerializer.textEncoder.encode(value));
  }

  public serializeBytes(value: Uint8Array): void {
    this.serializeLen(value.length);
    this.serialize(value);
  }

  public serializeBool(value: boolean): void {
    const byteValue = value ? 1 : 0;
    this.serialize(new Uint8Array([byteValue]));
  }

  // eslint-disable-next-line @typescript-eslint/no-unused-vars,@typescript-eslint/explicit-module-boundary-types
  public serializeUnit(_value: null): void {
    return;
  }

  private serializeWithFunction(
    fn: (byteOffset: number, value: number, littleEndian: boolean) => void,
    bytesLength: number,
    value: number,
  ) {
    this.ensureBufferWillHandleSize(bytesLength);
    const dv = new DataView(this.buffer, this.offset);
    fn.apply(dv, [0, value, true]);
    this.offset += bytesLength;
  }

  public serializeU8(value: number): void {
    this.serialize(new Uint8Array([value]));
  }

  public serializeU16(value: number): void {
    this.serializeWithFunction(DataView.prototype.setUint16, 2, value);
  }

  public serializeU32(value: number): void {
    this.serializeWithFunction(DataView.prototype.setUint32, 4, value);
  }

  public serializeU64(value: BigInt | number): void {
    const low = BigInt(value.toString()) & BinarySerializer.BIG_32Fs;
    const high = BigInt(value.toString()) >> BinarySerializer.BIG_32;

    // write little endian number
    this.serializeU32(Number(low));
    this.serializeU32(Number(high));
  }

  public serializeU128(value: BigInt | number): void {
    const low = BigInt(value.toString()) & BinarySerializer.BIG_64Fs;
    const high = BigInt(value.toString()) >> BinarySerializer.BIG_64;

    // write little endian number
    this.serializeU64(low);
    this.serializeU64(high);
  }

  public serializeI8(value: number): void {
    const bytes = 1;
    this.ensureBufferWillHandleSize(bytes);
    new DataView(this.buffer, this.offset).setInt8(0, value);
    this.offset += bytes;
  }

  public serializeI16(value: number): void {
    const bytes = 2;
    this.ensureBufferWillHandleSize(bytes);
    new DataView(this.buffer, this.offset).setInt16(0, value, true);
    this.offset += bytes;
  }

  public serializeI32(value: number): void {
    const bytes = 4;
    this.ensureBufferWillHandleSize(bytes);
    new DataView(this.buffer, this.offset).setInt32(0, value, true);
    this.offset += bytes;
  }

  public serializeI64(value: bigint | number): void {
    const low = BigInt(value) & BinarySerializer.BIG_32Fs;
    const high = BigInt(value) >> BinarySerializer.BIG_32;

    // write little endian number
    this.serializeI32(Number(low));
    this.serializeI32(Number(high));
  }

  public serializeI128(value: bigint | number): void {
    const low = BigInt(value) & BinarySerializer.BIG_64Fs;
    const high = BigInt(value) >> BinarySerializer.BIG_64;

    // write little endian number
    this.serializeI64(low);
    this.serializeI64(high);
  }

  public serializeOptionTag(value: boolean): void {
    this.serializeBool(value);
  }

  public getBufferOffset(): number {
    return this.offset;
  }

  public getBytes(): Uint8Array {
    return new Uint8Array(this.buffer).slice(0, this.offset);
  }

  public serializeChar(_value: string): void {
    throw new Error("Method serializeChar not implemented.");
  }

  public serializeF32(value: number): void {
    const bytes = 4;
    this.ensureBufferWillHandleSize(bytes);
    new DataView(this.buffer, this.offset).setFloat32(0, value, true);
    this.offset += bytes;
  }

  public serializeF64(value: number): void {
    const bytes = 8;
    this.ensureBufferWillHandleSize(bytes);
    new DataView(this.buffer, this.offset).setFloat64(0, value, true);
    this.offset += bytes;
  }
}

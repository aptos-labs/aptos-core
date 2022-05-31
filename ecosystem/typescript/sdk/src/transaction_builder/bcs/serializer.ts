export interface Serializer {
  serializeStr(value: string): void;

  serializeBytes(value: Uint8Array): void;

  serializeFixedBytes(value: Uint8Array): void;

  serializeBool(value: boolean): void;

  serializeU8(value: number): void;

  serializeU16(value: number): void;

  serializeU32(value: number): void;

  serializeU64(value: bigint | number): void;

  serializeU128(value: bigint | number): void;

  serializeLen(value: number): void;

  serializeVariantIndex(value: number): void;

  serializeOptionTag(value: boolean): void;

  getBufferOffset(): number;

  getBytes(): Uint8Array;
}

export class BcsSerializer implements Serializer {
  private static readonly BIG_MAX_U32: bigint = BigInt("4294967295");

  private static readonly BIG_MAX_U64: bigint = BigInt("18446744073709551615");

  private static readonly textEncoder = new TextEncoder();

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

  public serializeStr(value: string): void {
    this.serializeBytes(BcsSerializer.textEncoder.encode(value));
  }

  public serializeBytes(value: Uint8Array): void {
    this.serializeLen(value.length);
    this.serialize(value);
  }

  public serializeFixedBytes(value: Uint8Array): void {
    this.serialize(value);
  }

  public serializeBool(value: boolean): void {
    const byteValue = value ? 1 : 0;
    this.serialize(new Uint8Array([byteValue]));
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
    const low = BigInt(value.toString()) & BcsSerializer.BIG_MAX_U32;
    const high = BigInt(value.toString()) >> BigInt(32);

    // write little endian number
    this.serializeU32(Number(low));
    this.serializeU32(Number(high));
  }

  public serializeU128(value: BigInt | number): void {
    const low = BigInt(value.toString()) & BcsSerializer.BIG_MAX_U64;
    const high = BigInt(value.toString()) >> BigInt(64);

    // write little endian number
    this.serializeU64(low);
    this.serializeU64(high);
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

  public serializeU32AsUleb128(value: number): void {
    const valueArray = [];
    while (value >>> 7 !== 0) {
      valueArray.push((value & 0x7f) | 0x80);
      value >>>= 7;
    }
    valueArray.push(value);
    this.serialize(new Uint8Array(valueArray));
  }

  serializeLen(value: number): void {
    this.serializeU32AsUleb128(value);
  }

  public serializeVariantIndex(value: number): void {
    this.serializeU32AsUleb128(value);
  }
}

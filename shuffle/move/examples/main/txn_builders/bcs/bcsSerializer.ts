import { BinarySerializer } from "../serde/binarySerializer.ts";

export class BcsSerializer extends BinarySerializer {
  public serializeU32AsUleb128(value: number): void {
    const valueArray = [];
    while (value >>> 7 != 0) {
      valueArray.push((value & 0x7f) | 0x80);
      value = value >>> 7;
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

  public sortMapEntries(_offsets: number[]) {
    // leaving it empty for now, should be implemented soon
  }
}

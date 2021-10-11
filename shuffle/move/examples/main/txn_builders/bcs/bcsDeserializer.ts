import { BinaryDeserializer } from "../serde/binaryDeserializer.ts";

export class BcsDeserializer extends BinaryDeserializer {
  private static readonly MAX_UINT_32 = 2 ** 32 - 1;

  public deserializeUleb128AsU32(): number {
    let value = 0;
    for (let shift = 0; shift < 32; shift += 7) {
      const x = this.deserializeU8();
      const digit = x & 0x7f;
      value = value | (digit << shift);
      if (value < 0 || value > BcsDeserializer.MAX_UINT_32) {
        throw new Error("Overflow while parsing uleb128-encoded uint32 value");
      }
      if (digit == x) {
        if (shift > 0 && digit == 0) {
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

  public checkThatKeySlicesAreIncreasing(
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _key1: [number, number],
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _key2: [number, number],
  ): void {
    return;
  }
}

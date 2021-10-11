export interface Deserializer {
  deserializeStr(): string;

  deserializeBytes(): Uint8Array;

  deserializeBool(): boolean;

  deserializeUnit(): null;

  deserializeChar(): string;

  deserializeF32(): number;

  deserializeF64(): number;

  deserializeU8(): number;

  deserializeU16(): number;

  deserializeU32(): number;

  deserializeU64(): bigint;

  deserializeU128(): bigint;

  deserializeI8(): number;

  deserializeI16(): number;

  deserializeI32(): number;

  deserializeI64(): bigint;

  deserializeI128(): bigint;

  deserializeLen(): number;

  deserializeVariantIndex(): number;

  deserializeOptionTag(): boolean;

  getBufferOffset(): number;

  checkThatKeySlicesAreIncreasing(
    key1: [number, number],
    key2: [number, number],
  ): void;
}

import { Deserializer } from "./deserializer";
import { Serializer } from "./serializer";
import { bytes, Seq, uint128, uint16, uint32, uint64, uint8 } from "./types";

// Upper bound values for uint8, uint16, uint64 and uint128
export const MAX_U8_NUMBER: uint8 = 2 ** 8 - 1;
export const MAX_U16_NUMBER: uint16 = 2 ** 16 - 1;
export const MAX_U32_NUMBER: uint32 = 2 ** 32 - 1;
export const MAX_U64_BIG_INT: uint64 = BigInt(2 ** 64) - 1n;
export const MAX_U128_BIG_INT: uint128 = BigInt(2 ** 128) - 1n;

interface Serializable {
  serialize(serializer: Serializer): void;
}

/**
 * Serializes a vector values that are "Serializable".
 */
export function serializeVector<T extends Serializable>(value: Seq<T>, serializer: Serializer): void {
  serializer.serializeU32AsUleb128(value.length);
  value.forEach((item: T) => {
    item.serialize(serializer);
  });
}

/**
 * Deserializes a vector of values.
 */
export function deserializeVector(deserializer: Deserializer, cls: any): any[] {
  const length = deserializer.deserializeUleb128AsU32();
  const list: Seq<typeof cls> = [];
  for (let i = 0; i < length; i++) {
    list.push(cls.deserialize(deserializer));
  }
  return list;
}

export function bcsToBytes<T extends Serializable>(value: T): bytes {
  const serializer = new Serializer();
  value.serialize(serializer);
  return serializer.getBytes();
}

export function bcsSerializeUint64(value: bigint | number): bytes {
  const serializer = new Serializer();
  serializer.serializeU64(value);
  return serializer.getBytes();
}

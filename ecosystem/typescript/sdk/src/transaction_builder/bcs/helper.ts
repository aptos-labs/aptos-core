import { Deserializer } from './deserializer';
import { Serializer } from './serializer';
import { Bytes, Seq } from './types';

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
  for (let i = 0; i < length; i += 1) {
    list.push(cls.deserialize(deserializer));
  }
  return list;
}

export function bcsToBytes<T extends Serializable>(value: T): Bytes {
  const serializer = new Serializer();
  value.serialize(serializer);
  return serializer.getBytes();
}

export function bcsSerializeUint64(value: bigint | number): Bytes {
  const serializer = new Serializer();
  serializer.serializeU64(value);
  return serializer.getBytes();
}

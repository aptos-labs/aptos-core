import { Deserializer } from "./deserializer";
import { BcsSerializer, Serializer } from "./serializer";
import { bytes, Seq } from "./types";

interface Serializable {
  serialize(serializer: Serializer): void;
}

export function serializeVector<T extends Serializable>(value: Seq<T>, serializer: Serializer): void {
  serializer.serializeU32AsUleb128(value.length);
  value.forEach((item: T) => {
    item.serialize(serializer);
  });
}

export function deserializeVector(deserializer: Deserializer, cls: any) {
  const length = deserializer.deserializeUleb128AsU32();
  const list: Seq<typeof cls> = [];
  for (let i = 0; i < length; i++) {
    list.push(cls.deserialize(deserializer));
  }
  return list;
}

export function bcsToBytes<T extends Serializable>(value: T): bytes {
  const serializer = new BcsSerializer();
  value.serialize(serializer);
  return serializer.getBytes();
}

export function bcsSerializeUint64(value: BigInt | number): bytes {
  const serializer = new BcsSerializer();
  serializer.serializeU64(value);
  return serializer.getBytes();
}

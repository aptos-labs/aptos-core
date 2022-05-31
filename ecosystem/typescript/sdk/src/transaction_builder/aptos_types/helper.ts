import { Deserializer, Seq, Serializer } from "../bcs";

interface Serializable {
  serialize(serializer: Serializer): void;
}

export function serializeVector<T extends Serializable>(value: Seq<T>, serializer: Serializer): void {
  serializer.serializeLen(value.length);
  value.forEach((item: T) => {
    item.serialize(serializer);
  });
}

export function deserializeVector(deserializer: Deserializer, cls: any) {
  const length = deserializer.deserializeLen();
  const list: Seq<typeof cls> = [];
  for (let i = 0; i < length; i++) {
    list.push(cls.deserialize(deserializer));
  }
  return list;
}

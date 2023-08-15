// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "./deserializer";
import { Serializer } from "./serializer";
import { AnyNumber, Bytes, Seq, Uint16, Uint32, Uint8 } from "./types";

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
 * Serializes a vector with specified item serialization function.
 * Very dynamic function and bypasses static typechecking.
 */
export function serializeVectorWithFunc(value: any[], func: string): Bytes {
  const serializer = new Serializer();
  serializer.serializeU32AsUleb128(value.length);
  const f = (serializer as any)[func];
  value.forEach((item) => {
    f.call(serializer, item);
  });
  return serializer.getBytes();
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

export function bcsSerializeUint64(value: AnyNumber): Bytes {
  const serializer = new Serializer();
  serializer.serializeU64(value);
  return serializer.getBytes();
}

export function bcsSerializeU8(value: Uint8): Bytes {
  const serializer = new Serializer();
  serializer.serializeU8(value);
  return serializer.getBytes();
}

export function bcsSerializeU16(value: Uint16): Bytes {
  const serializer = new Serializer();
  serializer.serializeU16(value);
  return serializer.getBytes();
}

export function bcsSerializeU32(value: Uint32): Bytes {
  const serializer = new Serializer();
  serializer.serializeU32(value);
  return serializer.getBytes();
}

export function bcsSerializeU128(value: AnyNumber): Bytes {
  const serializer = new Serializer();
  serializer.serializeU128(value);
  return serializer.getBytes();
}

export function bcsSerializeU256(value: AnyNumber): Bytes {
  const serializer = new Serializer();
  serializer.serializeU256(value);
  return serializer.getBytes();
}

export function bcsSerializeBool(value: boolean): Bytes {
  const serializer = new Serializer();
  serializer.serializeBool(value);
  return serializer.getBytes();
}

export function bcsSerializeStr(value: string): Bytes {
  const serializer = new Serializer();
  serializer.serializeStr(value);
  return serializer.getBytes();
}

export function bcsSerializeBytes(value: Bytes): Bytes {
  const serializer = new Serializer();
  serializer.serializeBytes(value);
  return serializer.getBytes();
}

export function bcsSerializeFixedBytes(value: Bytes): Bytes {
  const serializer = new Serializer();
  serializer.serializeFixedBytes(value);
  return serializer.getBytes();
}

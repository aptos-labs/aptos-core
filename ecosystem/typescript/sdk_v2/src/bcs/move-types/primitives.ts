// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AnyNumber } from "../../types";
import { Deserializer } from "../deserializer";
import { Serializable, Serializer } from "../serializer";

export class Bool extends Serializable {
  constructor(public value: boolean) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBool(this.value);
  }

  deserialize(deserializer: Deserializer) {
    this.value = deserializer.deserializeBool();
  }

  static deserialize(deserializer: Deserializer): Bool {
    return new Bool(deserializer.deserializeBool());
  }
}
export class U8 extends Serializable {
  constructor(public value: number) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU8(this.value);
  }

  static deserialize(deserializer: Deserializer): U8 {
    return new U8(deserializer.deserializeU8());
  }
}
export class U16 extends Serializable {
  constructor(public value: number) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU16(this.value);
  }

  static deserialize(deserializer: Deserializer): U16 {
    return new U16(deserializer.deserializeU16());
  }
}
export class U32 extends Serializable {
  constructor(public value: number) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32(this.value);
  }

  static deserialize(deserializer: Deserializer): U32 {
    return new U32(deserializer.deserializeU32());
  }
}
export class U64 extends Serializable {
  value: bigint;

  constructor(value: AnyNumber) {
    super();
    this.value = BigInt(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU64(this.value);
  }

  static deserialize(deserializer: Deserializer): U64 {
    return new U64(deserializer.deserializeU64());
  }
}
export class U128 extends Serializable {
  value: bigint;

  constructor(value: AnyNumber) {
    super();
    this.value = BigInt(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU128(this.value);
  }

  static deserialize(deserializer: Deserializer): U128 {
    return new U128(deserializer.deserializeU128());
  }
}
export class U256 extends Serializable {
  value: bigint;

  constructor(value: AnyNumber) {
    super();
    this.value = BigInt(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU256(this.value);
  }

  static deserialize(deserializer: Deserializer): U256 {
    return new U256(deserializer.deserializeU256());
  }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import {
  MAX_U128_BIG_INT,
  MAX_U16_NUMBER,
  MAX_U32_NUMBER,
  MAX_U64_BIG_INT,
  MAX_U8_NUMBER,
  MAX_U256_BIG_INT,
} from "../consts";
import { AnyNumber, Uint16, Uint32, Uint8 } from "../../types";
import { Deserializer } from "../deserializer";
import { Serializable, Serializer, ensureBoolean, validateNumberInRange } from "../serializer";

export class Bool extends Serializable {
  public readonly value: boolean;

  constructor(value: boolean) {
    super();
    ensureBoolean(value);
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBool(this.value);
  }

  static deserialize(deserializer: Deserializer): Bool {
    return new Bool(deserializer.deserializeBool());
  }
}

export class U8 extends Serializable {
  public readonly value: Uint8;

  constructor(value: Uint8) {
    super();
    validateNumberInRange(value, 0, MAX_U8_NUMBER);
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU8(this.value);
  }

  static deserialize(deserializer: Deserializer): U8 {
    return new U8(deserializer.deserializeU8());
  }
}

export class U16 extends Serializable {
  public readonly value: Uint16;

  constructor(value: Uint16) {
    super();
    validateNumberInRange(value, 0, MAX_U16_NUMBER);
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU16(this.value);
  }

  static deserialize(deserializer: Deserializer): U16 {
    return new U16(deserializer.deserializeU16());
  }
}

export class U32 extends Serializable {
  public readonly value: Uint32;

  constructor(value: Uint32) {
    super();
    validateNumberInRange(value, 0, MAX_U32_NUMBER);
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32(this.value);
  }

  static deserialize(deserializer: Deserializer): U32 {
    return new U32(deserializer.deserializeU32());
  }
}

export class U64 extends Serializable {
  public readonly value: bigint;

  constructor(value: AnyNumber) {
    super();
    validateNumberInRange(value, BigInt(0), MAX_U64_BIG_INT);
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
  public readonly value: bigint;

  constructor(value: AnyNumber) {
    super();
    validateNumberInRange(value, BigInt(0), MAX_U128_BIG_INT);
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
  public readonly value: bigint;

  constructor(value: AnyNumber) {
    super();
    validateNumberInRange(value, BigInt(0), MAX_U256_BIG_INT);
    this.value = BigInt(value);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU256(this.value);
  }

  static deserialize(deserializer: Deserializer): U256 {
    return new U256(deserializer.deserializeU256());
  }
}

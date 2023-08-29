// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer, Serializer } from "../bcs";

export class Ed25519PublicKey {
  static readonly LENGTH: number = 32;

  readonly value: Uint8Array;

  constructor(value: Uint8Array) {
    if (value.length !== Ed25519PublicKey.LENGTH) {
      throw new Error(`Ed25519PublicKey length should be ${Ed25519PublicKey.LENGTH}`);
    }
    this.value = value;
  }

  toBytes(): Uint8Array {
    return this.value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeByteVector(this.value);
  }

  static deserialize(deserializer: Deserializer): Ed25519PublicKey {
    const value = deserializer.deserializeBytes();
    return new Ed25519PublicKey(value);
  }
}

export class Ed25519Signature {
  static readonly LENGTH = 64;

  constructor(public readonly value: Uint8Array) {
    if (value.length !== Ed25519Signature.LENGTH) {
      throw new Error(`Ed25519Signature length should be ${Ed25519Signature.LENGTH}`);
    }
  }

  serialize(serializer: Serializer): void {
    serializer.serializeByteVector(this.value);
  }

  static deserialize(deserializer: Deserializer): Ed25519Signature {
    const value = deserializer.deserializeBytes();
    return new Ed25519Signature(value);
  }
}

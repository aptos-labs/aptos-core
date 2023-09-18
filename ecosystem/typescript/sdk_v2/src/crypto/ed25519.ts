// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer, Serializer } from "../bcs";
import { Hex } from "../core";
import { HexInput } from "../types";

export class Ed25519PublicKey {
  // Correct length of the public key in bytes (Uint8Array)
  static readonly LENGTH: number = 32;

  // The public key in hex format
  readonly data: Hex;

  constructor(hexInput: HexInput) {
    const hex = Hex.fromHexInput({ hexInput });
    if (hex.toUint8Array().length !== Ed25519PublicKey.LENGTH) {
      throw new Error(`Ed25519PublicKey length should be ${Ed25519PublicKey.LENGTH}`);
    }
    this.data = hex;
  }

  toUint8Array(): Uint8Array {
    return this.data.toUint8Array();
  }

  toString(): string {
    return this.data.toString();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.data.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): Ed25519PublicKey {
    const value = deserializer.deserializeBytes();
    return new Ed25519PublicKey(value);
  }
}

export class Ed25519Signature {
  static readonly LENGTH = 64;

  public readonly data: Uint8Array;

  constructor(value: Uint8Array) {
    if (value.length !== Ed25519Signature.LENGTH) {
      throw new Error(`Ed25519Signature length should be ${Ed25519Signature.LENGTH}`);
    }
    this.data = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.data);
  }

  static deserialize(deserializer: Deserializer): Ed25519Signature {
    const value = deserializer.deserializeBytes();
    return new Ed25519Signature(value);
  }
}

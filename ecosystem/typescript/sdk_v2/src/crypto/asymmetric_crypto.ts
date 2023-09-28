// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializable, Deserializer, Serializable, Serializer } from "../bcs";
import { HexInput } from "../types";

export abstract class PublicKey implements Serializable, Deserializable<PublicKey> {
  // Verify the given message with the public key and signature.
  abstract verifySignature(args: { data: HexInput; signature: Signature }): boolean;

  // Convert the public key to bytes or Uint8Array.
  abstract toUint8Array(): Uint8Array;

  // Convert the public key to a hex string with the 0x prefix.
  abstract toString(): string;

  // TODO: This should be a static method.
  abstract deserialize(deserializer: Deserializer): PublicKey;
  abstract serialize(serializer: Serializer): void;
}

export abstract class PrivateKey implements Serializable, Deserializable<PrivateKey> {
  // Sign the given message with the private key.
  abstract sign(args: { message: HexInput }): Signature;

  // Convert the private key to bytes or Uint8Array.
  abstract toUint8Array(): Uint8Array;

  // Convert the private key to a hex string with the 0x prefix.
  abstract toString(): string;

  // TODO: This should be a static method.
  abstract deserialize(deserializer: Deserializer): PrivateKey;
  abstract serialize(serializer: Serializer): void;
}

export abstract class Signature implements Serializable, Deserializable<Signature> {
  // Convert the signature to bytes or Uint8Array.
  abstract toUint8Array(): Uint8Array;

  // Convert the signature to a hex string with the 0x prefix.
  abstract toString(): string;

  // TODO: This should be a static method.
  abstract deserialize(deserializer: Deserializer): Signature;
  abstract serialize(serializer: Serializer): void;
}

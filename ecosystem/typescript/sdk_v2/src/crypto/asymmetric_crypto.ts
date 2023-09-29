// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Serializable, Serializer } from "../bcs";
import { HexInput } from "../types";

/**
 * An abstract representation of a public key.  All Asymmetric key pairs will use this to
 * verify signatures and for authentication keys.
 */
export abstract class PublicKey extends Serializable {
  /**
   * Verifies that the private key associated with this public key signed the message with the given signature.
   * @param args
   */
  abstract verifySignature(args: { message: HexInput; signature: Signature }): boolean;

  /**
   * Get the raw public key bytes
   */
  abstract toUint8Array(): Uint8Array;

  /**
   * Get the public key as a hex string with a 0x prefix e.g. 0x123456...
   */
  abstract toString(): string;

  abstract serialize(serializer: Serializer): void;
}

/**
 * An abstract representation of a private key.  This is used to sign transactions and
 * derive the public key associated.
 */
export abstract class PrivateKey extends Serializable {
  /**
   * Sign a message with the key
   * @param args
   */
  abstract sign(args: { message: HexInput }): Signature;

  /**
   * Get the raw private key bytes
   */
  abstract toUint8Array(): Uint8Array;

  /**
   * Get the private key as a hex string with a 0x prefix e.g. 0x123456...
   */
  abstract toString(): string;

  abstract serialize(serializer: Serializer): void;

  /**
   * Derives the public key associated with the private key
   */
  abstract publicKey(): PublicKey;
}

/**
 * An abstract representation of a signature.  This is the product of signing a
 * message and can be used with the PublicKey to verify the signature.
 */
export abstract class Signature extends Serializable {
  /**
   * Get the raw signature bytes
   */
  abstract toUint8Array(): Uint8Array;

  /**
   * Get the signature as a hex string with a 0x prefix e.g. 0x123456...
   */
  abstract toString(): string;

  abstract serialize(serializer: Serializer): void;
}

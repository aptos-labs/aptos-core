// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Serializable } from "../bcs";
import { HexInput } from "../types";

export interface PublicKey extends Serializable {
  // Verify the given message with the public key and signature.
  verifySignature(args: { data: HexInput; signature: Signature }): boolean;

  // Convert the public key to bytes or Uint8Array.
  toUint8Array(): Uint8Array;

  // Convert the public key to a hex string with the 0x prefix.
  toString(): string;
}

export interface PrivateKey extends Serializable {
  // Sign the given message with the private key.
  sign(args: { message: HexInput }): Signature;

  // Convert the private key to bytes or Uint8Array.
  toUint8Array(): Uint8Array;

  // Convert the private key to a hex string with the 0x prefix.
  toString(): string;
}

export interface Signature extends Serializable {
  // Convert the signature to bytes or Uint8Array.
  toUint8Array(): Uint8Array;

  // Convert the signature to a hex string with the 0x prefix.
  toString(): string;
}

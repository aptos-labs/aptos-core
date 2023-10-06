// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { sha256 } from "@noble/hashes/sha256";
import { secp256k1 } from "@noble/curves/secp256k1";
import { Deserializer, Serializer } from "../bcs";
import { Hex } from "../core";
import { HexInput } from "../types";
import { PrivateKey, PublicKey, Signature } from "./asymmetric_crypto";

/**
 * Represents the Secp256k1 ecdsa public key
 */
export class Secp256k1PublicKey extends PublicKey {
  // Secp256k1 ecdsa public keys contain a prefix indicating compression and two 32-byte coordinates.
  static readonly LENGTH: number = 65;

  // Hex value of the public key
  private readonly key: Hex;

  /**
   * Create a new PublicKey instance from a Uint8Array or String.
   *
   * @param args.hexInput A HexInput (string or Uint8Array)
   */
  constructor(args: { hexInput: HexInput }) {
    super();

    const hex = Hex.fromHexInput(args);
    if (hex.toUint8Array().length !== Secp256k1PublicKey.LENGTH) {
      throw new Error(`PublicKey length should be ${Secp256k1PublicKey.LENGTH}`);
    }
    this.key = hex;
  }

  /**
   * Get the public key in bytes (Uint8Array).
   *
   * @returns Uint8Array representation of the public key
   */
  toUint8Array(): Uint8Array {
    return this.key.toUint8Array();
  }

  /**
   * Get the public key as a hex string with the 0x prefix.
   *
   * @returns string representation of the public key
   */
  toString(): string {
    return this.key.toString();
  }

  /**
   * Verifies a signed data with a public key
   *
   * @param args.message message
   * @param args.signature The signature
   * @returns true if the signature is valid
   */
  verifySignature(args: { message: HexInput; signature: Secp256k1Signature }): boolean {
    const { message, signature } = args;
    const msgHex = Hex.fromHexInput({ hexInput: message }).toUint8Array();
    const sha256Message = sha256(msgHex);
    const rawSignature = signature.toUint8Array();
    return secp256k1.verify(rawSignature, sha256Message, this.toUint8Array());
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.key.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): Secp256k1PublicKey {
    const bytes = deserializer.deserializeBytes();
    return new Secp256k1PublicKey({ hexInput: bytes });
  }
}

/**
 * A Secp256k1 ecdsa private key
 */
export class Secp256k1PrivateKey extends PrivateKey {
  /**
   * Length of Secp256k1 ecdsa private key
   */
  static readonly LENGTH: number = 32;

  /**
   * The private key bytes
   * @private
   */
  private readonly key: Hex;

  /**
   * Create a new PrivateKey instance from a Uint8Array or String.
   *
   * @param args.hexInput A HexInput (string or Uint8Array)
   */
  constructor(args: { hexInput: HexInput }) {
    super();

    const privateKeyHex = Hex.fromHexInput(args);
    if (privateKeyHex.toUint8Array().length !== Secp256k1PrivateKey.LENGTH) {
      throw new Error(`PrivateKey length should be ${Secp256k1PrivateKey.LENGTH}`);
    }

    this.key = privateKeyHex;
  }

  /**
   * Get the private key in bytes (Uint8Array).
   *
   * @returns
   */
  toUint8Array(): Uint8Array {
    return this.key.toUint8Array();
  }

  /**
   * Get the private key as a hex string with the 0x prefix.
   *
   * @returns string representation of the private key
   */
  toString(): string {
    return this.key.toString();
  }

  /**
   * Sign the given message with the private key.
   *
   * @param args.message in HexInput format
   * @returns Signature
   */
  sign(args: { message: HexInput }): Secp256k1Signature {
    const msgHex = Hex.fromHexInput({ hexInput: args.message });
    const sha256Message = sha256(msgHex.toUint8Array());
    const signature = secp256k1.sign(sha256Message, this.key.toUint8Array());
    return new Secp256k1Signature({ hexInput: signature.toCompactRawBytes() });
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): Secp256k1PrivateKey {
    const bytes = deserializer.deserializeBytes();
    return new Secp256k1PrivateKey({ hexInput: bytes });
  }

  /**
   * Generate a new random private key.
   *
   * @returns Secp256k1PrivateKey
   */
  static generate(): Secp256k1PrivateKey {
    const hexInput = secp256k1.utils.randomPrivateKey();
    return new Secp256k1PrivateKey({ hexInput });
  }

  /**
   * Derive the Secp256k1PublicKey from this private key.
   *
   * @returns Secp256k1PublicKey
   */
  publicKey(): Secp256k1PublicKey {
    const bytes = secp256k1.getPublicKey(this.key.toUint8Array(), false);
    return new Secp256k1PublicKey({ hexInput: bytes });
  }
}

/**
 * A signature of a message signed using an Secp256k1 ecdsa private key
 */
export class Secp256k1Signature extends Signature {
  /**
   * Secp256k1 ecdsa signatures are 256-bit.
   */
  static readonly LENGTH = 64;

  /**
   * The signature bytes
   * @private
   */
  private readonly data: Hex;

  /**
   * Create a new Signature instance from a Uint8Array or String.
   *
   * @param args.hexInput A HexInput (string or Uint8Array)
   */
  constructor(args: { hexInput: HexInput }) {
    super();

    const hex = Hex.fromHexInput(args);
    if (hex.toUint8Array().length !== Secp256k1Signature.LENGTH) {
      throw new Error(`Signature length should be ${Secp256k1Signature.LENGTH}`);
    }
    this.data = hex;
  }

  /**
   * Get the signature in bytes (Uint8Array).
   *
   * @returns Uint8Array representation of the signature
   */
  toUint8Array(): Uint8Array {
    return this.data.toUint8Array();
  }

  /**
   * Get the signature as a hex string with the 0x prefix.
   *
   * @returns string representation of the signature
   */
  toString(): string {
    return this.data.toString();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.data.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): Secp256k1Signature {
    const hex = deserializer.deserializeBytes();
    return new Secp256k1Signature({ hexInput: hex });
  }
}

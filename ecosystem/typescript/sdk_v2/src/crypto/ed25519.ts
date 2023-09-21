// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import nacl from "tweetnacl";
import { Deserializer } from "../bcs/deserializer";
import { Serializer } from "../bcs/serializer";
import { Hex } from "../core/hex";
import { HexInput } from "../types";

export class PublicKey {
  // Correct length of the public key in bytes (Uint8Array)
  static readonly LENGTH: number = 32;

  // Hex value of the public key
  private readonly key: Hex;

  /**
   * Create a new PublicKey instance from a Uint8Array or String.
   *
   * @param args.hexInput A HexInput (string or Uint8Array)
   */
  constructor(args: { hexInput: HexInput }) {
    const { hexInput } = args;
    const hex = Hex.fromHexInput({ hexInput });
    if (hex.toUint8Array().length !== PublicKey.LENGTH) {
      throw new Error(`PublicKey length should be ${PublicKey.LENGTH}`);
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
   * Verifies the signature of the message with the public key
   * @param args.message a signed message
   * @param args.signature the signature of the message
   */
  verifySignature(args: { message: HexInput; signature: HexInput }): boolean {
    const { message, signature } = args;
    const rawMessage = Hex.fromHexInput({ hexInput: message }).toUint8Array();
    const rawSignature = Hex.fromHexInput({ hexInput: signature }).toUint8Array();
    return nacl.sign.detached.verify(rawMessage, rawSignature, this.key.toUint8Array());
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.key.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): PublicKey {
    const value = deserializer.deserializeBytes();
    return new PublicKey({ hexInput: value });
  }
}

export class PrivateKey {
  // Correct length of the private key in bytes (Uint8Array)
  static readonly LENGTH: number = 32;

  // Private and public key pair
  private readonly signingKeyPair: nacl.SignKeyPair;

  /**
   * Create a new PrivateKey instance from a Uint8Array or String.
   *
   * @param value HexInput (string or Uint8Array)
   */
  constructor(args: { value: HexInput }) {
    const { value } = args;
    const privateKeyHex = Hex.fromHexInput({ hexInput: value });
    if (privateKeyHex.toUint8Array().length !== PrivateKey.LENGTH) {
      throw new Error(`PrivateKey length should be ${PrivateKey.LENGTH}`);
    }

    // Create keyPair from Private key in Uint8Array format
    const keyPair = nacl.sign.keyPair.fromSeed(privateKeyHex.toUint8Array().slice(0, 32));
    this.signingKeyPair = keyPair;
  }

  /**
   * Get the private key in bytes (Uint8Array).
   *
   * @returns Uint8Array representation of the private key
   */
  toUint8Array(): Uint8Array {
    return this.signingKeyPair.secretKey.slice(0, 32);
  }

  /**
   * Get the private key as a hex string with the 0x prefix.
   *
   * @returns string representation of the private key
   */
  toString(): string {
    return Hex.fromHexInput({ hexInput: this.signingKeyPair.secretKey.slice(0, 32) }).toString();
  }

  /**
   * Sign the given message with the private key.
   *
   * @param args.message in HexInput format
   * @returns Signature
   */
  sign(args: { message: HexInput }): Signature {
    const hex = Hex.fromHexInput({ hexInput: args.message });
    const signature = nacl.sign.detached(hex.toUint8Array(), this.signingKeyPair.secretKey);
    return new Signature({ value: signature });
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): PrivateKey {
    const value = deserializer.deserializeBytes();
    return new PrivateKey({ value });
  }
}

/**
 * The product of signing a message with a private key.
 */
export class Signature {
  // Correct length of the signature in bytes (Uint8Array)
  static readonly LENGTH = 64;

  // Hex value of the signature
  private readonly value: Hex;

  constructor(args: { value: HexInput }) {
    const hex = Hex.fromHexInput({ hexInput: args.value });
    if (hex.toUint8Array().length !== Signature.LENGTH) {
      throw new Error(`Signature length should be ${Signature.LENGTH}`);
    }

    this.value = hex;
  }

  /**
   * Get the signature in bytes (Uint8Array).
   *
   * @returns Uint8Array representation of the signature
   */
  toUint8Array(): Uint8Array {
    return this.value.toUint8Array();
  }

  /**
   * Get the signature as a hex string with the 0x prefix.
   *
   * @returns string representation of the signature
   */
  toString(): string {
    return this.value.toString();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.value.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): Signature {
    const value = deserializer.deserializeBytes();
    return new Signature({ value });
  }
}

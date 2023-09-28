// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../bcs/deserializer";
import { Serializer } from "../bcs/serializer";
import { Ed25519PublicKey, Ed25519Signature } from "./ed25519";
import { PublicKey, Signature } from "./asymmetric_crypto";
import { HexInput } from "../types";
import { Hex } from "../core/hex";

export class MultiEd25519PublicKey extends PublicKey {
  // Maximum number of public keys supported
  static readonly MAX_KEYS = 32;

  // Minimum number of public keys required
  static readonly MIN_KEYS = 2;

  // Minimum number of threshold supported
  static readonly MIN_THRESHOLD = 1;

  // List of Ed25519 public keys for this MultiEd25519PublicKey
  public readonly publicKeys: Ed25519PublicKey[];

  // The minimum number of valid signatures required, for the number of public keys specified
  public readonly threshold: number;

  /**
   * Public key for a K-of-N multisig transaction. A K-of-N multisig transaction means that for such a
   * transaction to be executed, at least K out of the N authorized signers have signed the transaction
   * and passed the check conducted by the chain.
   *
   * @see {@link
   * https://aptos.dev/integration/creating-a-signed-transaction/ | Creating a Signed Transaction}
   *
   * @param publicKeys A list of public keys
   * @param threshold At least "threshold" signatures must be valid
   */
  constructor(args: { publicKeys: Ed25519PublicKey[]; threshold: number }) {
    super();

    const { publicKeys, threshold } = args;

    // Validate number of public keys
    if (publicKeys.length > MultiEd25519PublicKey.MAX_KEYS || publicKeys.length < MultiEd25519PublicKey.MIN_KEYS) {
      throw new Error(
        `Must have between ${MultiEd25519PublicKey.MIN_KEYS} and ${MultiEd25519PublicKey.MAX_KEYS} public keys, inclusive`,
      );
    }

    // Validate threshold: must be between 1 and the number of public keys, inclusive
    if (threshold < MultiEd25519PublicKey.MIN_THRESHOLD || threshold > publicKeys.length) {
      throw new Error(
        `Threshold must be between ${MultiEd25519PublicKey.MIN_THRESHOLD} and ${publicKeys.length}, inclusive`,
      );
    }

    this.publicKeys = publicKeys;
    this.threshold = threshold;
  }

  /**
   * Converts a PublicKeys into Uint8Array (bytes) with: bytes = p1_bytes | ... | pn_bytes | threshold
   */
  toUint8Array(): Uint8Array {
    const bytes = new Uint8Array(this.publicKeys.length * Ed25519PublicKey.LENGTH + 1);
    this.publicKeys.forEach((k: Ed25519PublicKey, i: number) => {
      bytes.set(k.toUint8Array(), i * Ed25519PublicKey.LENGTH);
    });

    bytes[this.publicKeys.length * Ed25519PublicKey.LENGTH] = this.threshold;

    return bytes;
  }

  toString(): string {
    return Hex.fromHexInput({ hexInput: this.toUint8Array() }).toString();
  }

  verifySignature(args: { data: HexInput; signature: MultiEd25519Signature }): boolean {
    throw new Error("TODO - Method not implemented.");
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  // TODO: Update this in interface to be static, then remove this method
  deserialize(deserializer: Deserializer): PublicKey {
    throw new Error("Method not implemented.");
  }

  static deserialize(deserializer: Deserializer): MultiEd25519PublicKey {
    const bytes = deserializer.deserializeBytes();
    const threshold = bytes[bytes.length - 1];

    const keys: Ed25519PublicKey[] = [];

    for (let i = 0; i < bytes.length - 1; i += Ed25519PublicKey.LENGTH) {
      const begin = i;
      keys.push(new Ed25519PublicKey({ hexInput: bytes.subarray(begin, begin + Ed25519PublicKey.LENGTH) }));
    }
    return new MultiEd25519PublicKey({ publicKeys: keys, threshold });
  }
}

export class MultiEd25519Signature extends Signature {
  // Maximum number of signatures supported
  static MAX_SIGNATURES_SUPPORTED = 32;

  // Bitmap length
  static BITMAP_LEN: number = 4;

  // List of Ed25519Signatures for this MultiEd25519Signature
  public readonly signatures: Ed25519Signature[];

  // The bitmap masks that public key that has signed the message
  public readonly bitmap: Uint8Array;

  /**
   * Signature for a K-of-N multisig transaction.
   *
   * @see {@link
   * https://aptos.dev/integration/creating-a-signed-transaction/#multisignature-transactions | Creating a Signed Transaction}
   *
   * @param args.signatures A list of signatures
   * @param args.bitmap 4 bytes, at most 32 signatures are supported. If Nth bit value is `1`, the Nth
   * signature should be provided in `signatures`. Bits are read from left to right
   */
  constructor(args: { signatures: Ed25519Signature[]; bitmap: Uint8Array }) {
    super();

    const { signatures, bitmap } = args;
    if (bitmap.length !== MultiEd25519Signature.BITMAP_LEN) {
      throw new Error(`"bitmap" length should be ${MultiEd25519Signature.BITMAP_LEN}`);
    }

    if (signatures.length > MultiEd25519Signature.MAX_SIGNATURES_SUPPORTED) {
      throw new Error(
        `The number of signatures cannot be greater than ${MultiEd25519Signature.MAX_SIGNATURES_SUPPORTED}`,
      );
    }

    if (signatures.length > MultiEd25519Signature.MAX_SIGNATURES_SUPPORTED) {
      throw new Error(
        `The number of signatures cannot be greater than ${MultiEd25519Signature.MAX_SIGNATURES_SUPPORTED}`,
      );
    }

    this.signatures = signatures;
    this.bitmap = bitmap;
  }

  /**
   * Converts a MultiSignature into Uint8Array (bytes) with `bytes = s1_bytes | ... | sn_bytes | bitmap`
   */
  toUint8Array(): Uint8Array {
    const bytes = new Uint8Array(this.signatures.length * Ed25519Signature.LENGTH + MultiEd25519Signature.BITMAP_LEN);
    this.signatures.forEach((k: Ed25519Signature, i: number) => {
      bytes.set(k.toUint8Array(), i * Ed25519Signature.LENGTH);
    });

    bytes.set(this.bitmap, this.signatures.length * Ed25519Signature.LENGTH);

    return bytes;
  }

  toString(): string {
    return Hex.fromHexInput({ hexInput: this.toUint8Array() }).toString();
  }

  /**
   * Helper method to create a bitmap out of the specified bit positions
   * @param bits The bitmap positions that should be set. A position starts at index 0.
   * Valid position should range between 0 and 31.
   * @example
   * Here's an example of valid `bits`
   * ```
   * [0, 2, 31]
   * ```
   * `[0, 2, 31]` means the 1st, 3rd and 32nd bits should be set in the bitmap.
   * The result bitmap should be 0b1010000000000000000000000000001
   *
   * @returns bitmap that is 32bit long
   */
  static createBitmap(args: { bits: number[] }): Uint8Array {
    const { bits } = args;
    // Bits are read from left to right. e.g. 0b10000000 represents the first bit is set in one byte.
    // The decimal value of 0b10000000 is 128.
    const firstBitInByte = 128;
    const bitmap = new Uint8Array([0, 0, 0, 0]);

    // Check if duplicates exist in bits
    const dupCheckSet = new Set();

    bits.forEach((bit: number) => {
      if (bit >= MultiEd25519Signature.MAX_SIGNATURES_SUPPORTED) {
        throw new Error(`Cannot have a signature larger than ${MultiEd25519Signature.MAX_SIGNATURES_SUPPORTED - 1}.`);
      }

      if (dupCheckSet.has(bit)) {
        throw new Error("Duplicate bits detected.");
      }

      dupCheckSet.add(bit);

      const byteOffset = Math.floor(bit / 8);

      let byte = bitmap[byteOffset];

      // eslint-disable-next-line no-bitwise
      byte |= firstBitInByte >> bit % 8;

      bitmap[byteOffset] = byte;
    });

    return bitmap;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  // TODO: Update this in interface to be static, then remove this method
  deserialize(deserializer: Deserializer): Signature {
    throw new Error("Method not implemented.");
  }

  static deserialize(deserializer: Deserializer): MultiEd25519Signature {
    const bytes = deserializer.deserializeBytes();
    const bitmap = bytes.subarray(bytes.length - 4);

    const signatures: Ed25519Signature[] = [];

    for (let i = 0; i < bytes.length - bitmap.length; i += Ed25519Signature.LENGTH) {
      const begin = i;
      signatures.push(new Ed25519Signature({ data: bytes.subarray(begin, begin + Ed25519Signature.LENGTH) }));
    }
    return new MultiEd25519Signature({ signatures, bitmap });
  }
}

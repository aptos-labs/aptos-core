// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../bcs/deserializer";
import { Serializer } from "../bcs/serializer";
import { PublicKey, Signature } from "./ed25519";

export class MultiPublicKey {
  // Maximum number of public keys supported
  static readonly MAX_KEYS = 32;

  // Minimum number of public keys required
  static readonly MIN_KEYS = 2;

  // Minimum number of threshold supported
  static readonly MIN_THRESHOLD = 1;

  // List of Ed25519 public keys for this MultiEd25519PublicKey
  public readonly publicKeys: PublicKey[];

  // At least "threshold" signatures must be valid for the number of public keys specified
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
  constructor(args: { publicKeys: PublicKey[]; threshold: number }) {
    const { publicKeys, threshold } = args;

    // Validate number of public keys
    if (publicKeys.length > MultiPublicKey.MAX_KEYS || publicKeys.length < MultiPublicKey.MIN_KEYS) {
      throw new Error(
        `Must have between ${MultiPublicKey.MIN_KEYS} and ${MultiPublicKey.MAX_KEYS} public keys, inclusive`,
      );
    }

    // Validate threshold: must be between 1 and the number of public keys, inclusive
    if (threshold < MultiPublicKey.MIN_THRESHOLD || threshold > publicKeys.length) {
      throw new Error(`Threshold must be between ${MultiPublicKey.MIN_THRESHOLD} and ${publicKeys.length}, inclusive`);
    }

    this.publicKeys = publicKeys;
    this.threshold = threshold;
  }

  /**
   * Converts a PublicKeys into Uint8Array (bytes) with: bytes = p1_bytes | ... | pn_bytes | threshold
   */
  toUint8Array(): Uint8Array {
    const bytes = new Uint8Array(this.publicKeys.length * PublicKey.LENGTH + 1);
    this.publicKeys.forEach((k: PublicKey, i: number) => {
      bytes.set(k.toUint8Array(), i * PublicKey.LENGTH);
    });

    bytes[this.publicKeys.length * PublicKey.LENGTH] = this.threshold;

    return bytes;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): MultiPublicKey {
    const bytes = deserializer.deserializeBytes();
    const threshold = bytes[bytes.length - 1];

    const keys: PublicKey[] = [];

    for (let i = 0; i < bytes.length - 1; i += PublicKey.LENGTH) {
      const begin = i;
      keys.push(new PublicKey({ hexInput: bytes.subarray(begin, begin + PublicKey.LENGTH) }));
    }
    return new MultiPublicKey({ publicKeys: keys, threshold });
  }
}

export class MultiSignature {
  // Maximum number of signatures supported
  static MAX_SIGNATURES_SUPPORTED = 32;

  // Bitmap length
  static BITMAP_LEN: number = 4;

  public readonly signatures: Signature[];

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
  constructor(args: { signatures: Signature[]; bitmap: Uint8Array }) {
    const { signatures, bitmap } = args;
    if (bitmap.length !== MultiSignature.BITMAP_LEN) {
      throw new Error(`"bitmap" length should be ${MultiSignature.BITMAP_LEN}`);
    }

    this.signatures = signatures;
    this.bitmap = bitmap;
  }

  /**
   * Converts a MultiSignature into Uint8Array (bytes) with `bytes = s1_bytes | ... | sn_bytes | bitmap`
   */
  toUint8Array(): Uint8Array {
    const bytes = new Uint8Array(this.signatures.length * Signature.LENGTH + MultiSignature.BITMAP_LEN);
    this.signatures.forEach((k: Signature, i: number) => {
      bytes.set(k.toUint8Array(), i * Signature.LENGTH);
    });

    bytes.set(this.bitmap, this.signatures.length * Signature.LENGTH);

    return bytes;
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
  static createBitmap(bits: number[]): Uint8Array {
    // Bits are read from left to right. e.g. 0b10000000 represents the first bit is set in one byte.
    // The decimal value of 0b10000000 is 128.
    const firstBitInByte = 128;
    const bitmap = new Uint8Array([0, 0, 0, 0]);

    // Check if duplicates exist in bits
    const dupCheckSet = new Set();

    bits.forEach((bit: number) => {
      if (bit >= MultiSignature.MAX_SIGNATURES_SUPPORTED) {
        throw new Error(`Invalid bit value ${bit}.`);
      }

      if (dupCheckSet.has(bit)) {
        throw new Error("Duplicated bits detected.");
      }

      dupCheckSet.add(bit);

      const byteOffset = Math.floor(bit / 8);

      let byte = bitmap[byteOffset];

      byte |= firstBitInByte >> bit % 8;

      bitmap[byteOffset] = byte;
    });

    return bitmap;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): MultiSignature {
    const bytes = deserializer.deserializeBytes();
    const bitmap = bytes.subarray(bytes.length - 4);

    const sigs: Signature[] = [];

    for (let i = 0; i < bytes.length - bitmap.length; i += Signature.LENGTH) {
      const begin = i;
      sigs.push(new Signature({ data: bytes.subarray(begin, begin + Signature.LENGTH) }));
    }
    return new MultiSignature({ signatures: sigs, bitmap });
  }
}

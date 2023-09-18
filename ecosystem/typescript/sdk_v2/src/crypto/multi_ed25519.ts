// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable no-bitwise */
import { Deserializer, Serializer } from "../bcs";
import { Uint8 } from "../types";
import { Ed25519PublicKey, Ed25519Signature } from "./ed25519";

export class MultiEd25519PublicKey {
  // Maximum number of signatures shreshold supported
  static readonly MAX_SIGNATURES_SUPPORTED = 32;

  // List of Ed25519 public keys for this MultiEd25519PublicKey
  public readonly publicKeys: Ed25519PublicKey[];

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
   * @param public_keys A list of public keys
   * @param threshold At least "threshold" signatures must be valid
   */
  constructor(args: { publieKeys: Ed25519PublicKey[], threshold: number }) {
    const { publieKeys, threshold } = args;
    if (threshold > MultiEd25519PublicKey.MAX_SIGNATURES_SUPPORTED) {
      throw new Error(
        `"threshold" cannot be larger than ${MultiEd25519PublicKey.MAX_SIGNATURES_SUPPORTED}`,
      );
    }

    if (threshold < 0) {
      throw new Error(
        '"threshold" cannot be less than 0',
      );
    }

    this.publicKeys = publieKeys;
    this.threshold = threshold;
  }

  /**
   * Converts a MultiEd25519PublicKey into Uint8Array (bytes) with: bytes = p1_bytes | ... | pn_bytes | threshold
   */
  toUint8Array(): Uint8Array {
    const bytes = new Uint8Array(this.publicKeys.length * Ed25519PublicKey.LENGTH + 1);
    this.publicKeys.forEach((k: Ed25519PublicKey, i: number) => {
      bytes.set(k.data.toUint8Array(), i * Ed25519PublicKey.LENGTH);
    });

    bytes[this.publicKeys.length * Ed25519PublicKey.LENGTH] = this.threshold;

    return bytes;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toUint8Array());
  }

  static deserialize(deserializer: Deserializer): MultiEd25519PublicKey {
    const bytes = deserializer.deserializeBytes();
    const threshold = bytes[bytes.length - 1];

    const keys: Ed25519PublicKey[] = [];

    for (let i = 0; i < bytes.length - 1; i += Ed25519PublicKey.LENGTH) {
      const begin = i;
      keys.push(new Ed25519PublicKey(bytes.subarray(begin, begin + Ed25519PublicKey.LENGTH)));
    }
    return new MultiEd25519PublicKey({ publieKeys: keys, threshold });
  }
}

export class MultiEd25519Signature {
  static MAX_SIGNATURES_SUPPORTED = 32;
  static BITMAP_LEN: Uint8 = 4;

  public readonly signatures: Ed25519Signature[];

  public readonly bitmap: Uint8Array;

  /**
   * Signature for a K-of-N multisig transaction.
   *
   * @see {@link
   * https://aptos.dev/guides/creating-a-signed-transaction#multisignature-transactions | Creating a Signed Transaction}
   *
   * @param signatures A list of ed25519 signatures
   * @param bitmap 4 bytes, at most 32 signatures are supported. If Nth bit value is `1`, the Nth
   * signature should be provided in `signatures`. Bits are read from left to right
   */
  constructor(signatures: Ed25519Signature[], bitmap: Uint8Array) {
    if (bitmap.length !== MultiEd25519Signature.BITMAP_LEN) {
      throw new Error(`"bitmap" length should be ${MultiEd25519Signature.BITMAP_LEN}`);
    }

    this.signatures = signatures;
    this.bitmap = bitmap;
  }

  /**
   * Converts a MultiEd25519Signature into Uint8Array (bytes) with `bytes = s1_bytes | ... | sn_bytes | bitmap`
   */
  toUint8Array(): Uint8Array {
    const bytes = new Uint8Array(this.signatures.length * Ed25519Signature.LENGTH + MultiEd25519Signature.BITMAP_LEN);
    this.signatures.forEach((k: Ed25519Signature, i: number) => {
      bytes.set(k.data, i * Ed25519Signature.LENGTH);
    });

    bytes.set(this.bitmap, this.signatures.length * Ed25519Signature.LENGTH);

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
  static createBitmap(bits: Uint8[]): Uint8Array {
    // Bits are read from left to right. e.g. 0b10000000 represents the first bit is set in one byte.
    // The decimal value of 0b10000000 is 128.
    const firstBitInByte = 128;
    const bitmap = new Uint8Array([0, 0, 0, 0]);

    // Check if duplicates exist in bits
    const dupCheckSet = new Set();

    bits.forEach((bit: number) => {
      if (bit >= MultiEd25519Signature.MAX_SIGNATURES_SUPPORTED) {
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

  static deserialize(deserializer: Deserializer): MultiEd25519Signature {
    const bytes = deserializer.deserializeBytes();
    const bitmap = bytes.subarray(bytes.length - 4);

    const sigs: Ed25519Signature[] = [];

    for (let i = 0; i < bytes.length - bitmap.length; i += Ed25519Signature.LENGTH) {
      const begin = i;
      sigs.push(new Ed25519Signature(bytes.subarray(begin, begin + Ed25519Signature.LENGTH)));
    }
    return new MultiEd25519Signature(sigs, bitmap);
  }
}

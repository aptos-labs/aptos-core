// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it, vi } from "vitest";
import { hash_to_fq, hash_to_fr, hmac_kdf, SymmetricCiphertext, SymmetricKey, Test} from '../src/symmetric';
import { randomBytes } from '@noble/ciphers/utils.js';
import { gcm } from '@noble/ciphers/aes.js';
import { bn254 } from '@noble/curves/bn254.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { hash_to_field } from '@noble/curves/abstract/hash-to-curve.js'
import { Serializable, Serializer, Deserializer } from "@aptos-labs/ts-sdk";
import { warn } from "console";
import { H2COpts } from "@noble/curves/abstract/hash-to-curve.js";
import { leBytesToBigint, bigintToLEBytes } from "../src/util.js";


describe("util", () => {
  it("toLEBytes", () => {
    let bytes = randomBytes(32);
    let num : bigint = BigInt(0);

    for (let i = 0; i < bytes.length; i++) {
      let base : bigint = 256n ** BigInt(i);
      num += base * BigInt(bytes[i]);
    }

    let bytes_ = bigintToLEBytes(num);

    expect(bytes).toStrictEqual(bytes_);

  });

  it("to and from LE bytes", () => {
    let bytes = randomBytes(32);
    let num : bigint = BigInt(0);

    for (let i = 0; i < bytes.length; i++) {
      let base : bigint = 256n ** BigInt(i);
      num += base * BigInt(bytes[i]);
    }

    let bytes_ = bigintToLEBytes(num);

    let num_from_bytes_ = leBytesToBigint(bytes_);

    expect(num_from_bytes_).toStrictEqual(num);
  });
});

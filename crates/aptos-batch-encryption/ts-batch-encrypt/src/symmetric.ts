// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { randomBytes } from '@noble/ciphers/utils.js';
import { gcm } from '@noble/ciphers/aes.js';
import { Serializable, Serializer, Deserializer } from "@aptos-labs/ts-sdk";
import { hkdf } from '@noble/hashes/hkdf.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { type H2COpts, hash_to_field } from '@noble/curves/abstract/hash-to-curve.js';
import { bls12_381 } from '@noble/curves/bls12-381.js';
import { leBytesToBigint } from './fieldSerialization.ts';
import { type WeierstrassPoint } from '@noble/curves/abstract/weierstrass.js';
import type { Fp2 } from '@noble/curves/abstract/tower.js';
import { g2ToBytes } from './curveSerialization.ts';

// Domain separation tag for hash-to-curve.
// This must be identical between Rust and TypeScript implementations.
const HASH_G2_ELEMENT_DST = "APTOS_BATCH_ENCRYPTION_HASH_G2_ELEMENT";

export class Test extends Serializable {
  s: string;
  constructor(s: string) { super(); this.s = s; }

  serialize(serializer: Serializer): void {
    serializer.serializeStr(this.s);
  }

  static deserialize(deserializer: Deserializer): Test {
    return new Test(deserializer.deserializeStr());
  }
}

export class OneTimePad extends Serializable {
  otp: Uint8Array;

  constructor(otp: Uint8Array) {
    super();
    if (otp.length != 16) {
      throw new Error("One-time-pad length must be 16 bytes");
    }
    this.otp = otp;
  }

  static from_source_bytes(otp_source: Uint8Array): OneTimePad {
    let otp = hmac_kdf(otp_source);
    return new OneTimePad(otp.slice(0, 16));
  }

  pad_key(value: SymmetricKey): SymmetricKey {
    let paddedKey = [];
    for (let i = 0; i < 16; i++) {
      paddedKey.push(value.key[i] ^ this.otp[i]);
    }
    return new SymmetricKey(Uint8Array.from(paddedKey));
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.otp);
  }

  static deserialize(deserializer: Deserializer): OneTimePad {
    const otp: Uint8Array = deserializer.deserializeFixedBytes(16);
    return new OneTimePad(otp);
  }
}




export class SymmetricCiphertext extends Serializable {
  nonce: Uint8Array;
  ct_body: Uint8Array;

  constructor(nonce: Uint8Array, ct_body: Uint8Array) {
    super();
    this.nonce = nonce;
    this.ct_body = ct_body;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.nonce);
    serializer.serializeBytes(this.ct_body);
  }

  static deserialize(deserializer: Deserializer): SymmetricCiphertext {
    const nonce: Uint8Array = deserializer.deserializeFixedBytes(12);
    const ct_body: Uint8Array = deserializer.deserializeBytes();
    return new SymmetricCiphertext(nonce, ct_body);
  }
}


export class SymmetricKey extends Serializable {
  key: Uint8Array;

  constructor(key?: Uint8Array) {
    super();
    if (key) {
      if (key.length != 16) {
        throw new Error("Must provide a key of size 16")
      }
      this.key = key;
    } else {
      this.key = randomBytes(16);
    }
  }

  encrypt(msg: Serializable): SymmetricCiphertext {
    const nonce = randomBytes(12);

    var serializer = new Serializer();
    msg.serialize(serializer);
    const bytes = serializer.toUint8Array();
    

    const ct_body = gcm(this.key, nonce).encrypt(bytes);

    return new SymmetricCiphertext(nonce, ct_body);
  }

  serialize(serializer: Serializer): void {
    serializer.serializeFixedBytes(this.key);
  }

  static deserialize(deserializer: Deserializer): SymmetricKey {
    const key : Uint8Array = deserializer.deserializeFixedBytes(16);
    return new SymmetricKey(key);
  }
}

// Domain separation salt for the OTP KDF.
// This must be identical between Rust and TypeScript implementations.
const HKDF_SALT = new TextEncoder().encode("APTOS_BATCH_ENCRYPTION_OTP");

/**
 * Derives a 32-byte key from high-entropy source bytes using HKDF (RFC 5869).
 */
export function hmac_kdf(otp_source: Uint8Array): Uint8Array {
  return hkdf(sha256, otp_source, HKDF_SALT, new Uint8Array(), 32);
}


export function get_random_fr(): bigint {
  const random_bigint = leBytesToBigint(randomBytes(128));
  return bls12_381.G1.Point.Fn.create(random_bigint);
}

export function hash_to_fr(input: Uint8Array): bigint {
  const options : H2COpts = {
    DST: "",
    expand: "xmd",
    hash: sha256,
    p: bls12_381.fields.Fr.ORDER,
    m: 1,
    k: 128  
  }
  return hash_to_field(Uint8Array.from(input), 1, options)[0][0];
}

export function hash_to_fq(input: Uint8Array) {
  const options : H2COpts = {
    DST: "",
    expand: "xmd",
    hash: sha256,
    p: bls12_381.fields.Fp.ORDER,
    m: 1,
    k: 128  
  }
  return hash_to_field(Uint8Array.from(input), 1, options)[0][0];
}


export function hash_g2_element(g2_element: WeierstrassPoint<Fp2>): WeierstrassPoint<bigint> {
  const bytes = g2ToBytes(g2_element);
  return bls12_381.G1.hashToCurve(bytes, { DST: HASH_G2_ELEMENT_DST });
}

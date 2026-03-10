// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { Deserializer, Serializable, Serializer} from '@aptos-labs/ts-sdk';
import { hmac_kdf, hash_to_fq, hash_to_fr, SymmetricKey, Test, OneTimePad, hash_g2_element } from './symmetric.ts';
import { leBytesToBigint, bigintToLEBytesFr, bigintToLEBytesFq, leBytesToFp12, fp12ToLEBytes } from './fieldSerialization.ts';
import { bls12_381 } from '@noble/curves/bls12-381.js';
import { bytesToG2, g1ToBytes, g2ToBytes } from './curveSerialization.ts';
import { BIBECiphertext, EncryptionKey } from './ciphertext.ts';
import * as ed from '@noble/ed25519';

class TestEd25519 extends Serializable {
  secretKey: Uint8Array;
  publicKey: Uint8Array;
  msg: Uint8Array;
  signature: Uint8Array;

  constructor(secretKey: Uint8Array, publicKey: Uint8Array, msg: Uint8Array, signature: Uint8Array) {
    super();
    this.secretKey = secretKey;
    this.publicKey = publicKey;
    this.msg = msg;
    this.signature = signature;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.secretKey);
    serializer.serializeBytes(this.publicKey);
    serializer.serializeBytes(this.msg);
    serializer.serializeFixedBytes(this.signature);
  }

  static deserialize(deserializer: Deserializer): TestEd25519 {
    const secretKey = deserializer.deserializeBytes();
    const publicKey = deserializer.deserializeBytes();
    const msg = deserializer.deserializeBytes();
    const signature = deserializer.deserializeFixedBytes(64);
    return new TestEd25519(secretKey, publicKey, msg, signature);
  }
}

const functions = {
  'hmac_kdf': hmac_kdf,
  'hash_to_fr': function(x: Uint8Array) { return bigintToLEBytesFr(hash_to_fr(x)) },
  'hash_to_fq': function(x: Uint8Array) { return bigintToLEBytesFq(hash_to_fq(x)) },
  "symmetric_key_serialize":  function(x: Uint8Array) {
    const key = new SymmetricKey(x);
    var serializer = new Serializer();
    key.serialize(serializer);
    return serializer.toUint8Array();
  },
  "symmetric_encrypt":  function(x: Uint8Array) {
    const key = new SymmetricKey(x);
    const ct = key.encrypt(new Test("hi"));
    var serializer = new Serializer();
    ct.serialize(serializer);
    return serializer.toUint8Array();
  },
  "otp_generation":  function(x: Uint8Array) {
    const otp = OneTimePad.from_source_bytes(x);
    var serializer = new Serializer();
    otp.serialize(serializer);
    return serializer.toUint8Array();
  },
  "otp_padding":  function(x: Uint8Array) {
    let key_bytes = x.slice(0,16);
    let otp_bytes = x.slice(16,96);
    const key = new SymmetricKey(key_bytes);
    const otp = OneTimePad.from_source_bytes(otp_bytes);
    const padded_key = otp.pad_key(key);
    var serializer = new Serializer();
    padded_key.serialize(serializer);
    return serializer.toUint8Array();
  },
  "g1_serialization": function(x: Uint8Array) {
    let rand_exponent : bigint = leBytesToBigint(x);
    let g1 = bls12_381.G1.Point.BASE.multiply(rand_exponent);
    return g1ToBytes(g1);
  },
  "g2_serialization": function(x: Uint8Array) {
    let rand_exponent : bigint = leBytesToBigint(x);
    let g2 = bls12_381.G2.Point.BASE.multiply(rand_exponent);
    return g2ToBytes(g2);
  },
  "hash_g2_element": function(g2_bytes: Uint8Array) {
    let g2 = bytesToG2(g2_bytes);
    let hashed_g1 = hash_g2_element(g2);
    return g1ToBytes(hashed_g1);
  },
  "leBytesToFp12": function(bytes: Uint8Array) {
    let f = leBytesToFp12(bytes);
    let result = bls12_381.fields.Fp12.add(f, f);
    return fp12ToLEBytes(result);
  },
  "bibe_ciphertext_serialization": function(bytes: Uint8Array) {
    let ct = BIBECiphertext.deserialize(new Deserializer(bytes));

    if (!ct.ct_g2[0].equals(bls12_381.G2.Point.BASE)
    || !ct.ct_g2[1].equals(bls12_381.G2.Point.BASE.multiply(2n))
    || !ct.ct_g2[2].equals(bls12_381.G2.Point.BASE.multiply(3n))
    ) {
      throw new Error("incorrect ct_g2");
    }

    var serializer = new Serializer();
    ct.serialize(serializer);
    return serializer.toUint8Array();
  },
  "bibe_ciphertext_encrypt": function(bytes: Uint8Array) {
    let ek = EncryptionKey.deserialize(new Deserializer(bytes));

    let bibe_ct = ek.bibe_encrypt(new Test("hi"), 1n);

    var serializer = new Serializer();
    bibe_ct.serialize(serializer);
    return serializer.toUint8Array();
  },
  "ed25519": function(bytes: Uint8Array) {
    const args = TestEd25519.deserialize(new Deserializer(bytes));
    if (!ed.verify(args.signature, args.msg, args.publicKey)) {
      throw new Error("signature verification failed");
    }

    let signature = ed.sign(args.msg, args.secretKey);

    return signature;
  },
  "ciphertext_encrypt": function(bytes: Uint8Array) {
    let ek = EncryptionKey.deserialize(new Deserializer(bytes));

    let ct = ek.encrypt(new Test("hi"), new Test("associated data"));

    var serializer = new Serializer();
    ct.serialize(serializer);
    return serializer.toUint8Array();
  }
}

async function readStdin(): Promise<Uint8Array> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) chunks.push(chunk as Buffer);
  return Buffer.concat(chunks);
}

(async () => {
  try {
    const input : Uint8Array = await readStdin();
    console.error(input);
    const fnName = process.argv[3];
    const fn = functions[fnName];
    process.stdout.write(fn(input));
  } catch (err: any) {
    process.stderr.write(String(err?.stack || err));
    process.exit(1);
  }
})();

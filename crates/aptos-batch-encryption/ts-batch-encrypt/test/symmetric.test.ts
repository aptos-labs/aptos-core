// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it, } from "vitest";
import { hash_to_fq, hash_to_fr, hmac_kdf, SymmetricCiphertext, SymmetricKey, Test} from '../src/symmetric';
import { randomBytes } from '@noble/ciphers/utils.js';
import { gcm } from '@noble/ciphers/aes.js';
import { bn254 } from '@noble/curves/bn254.js';
import { Serializer, Deserializer } from "@aptos-labs/ts-sdk";
import { bytesToG2 } from "../src/curveSerialization.js";
import { hash_g2_element } from "../src/symmetric.js";


describe("Symmetric Crypto", () => {
  it("asdf", () => {
    const plaintext = new Uint8Array(32).fill(1);
    const key = randomBytes(16); // 24 for AES-192, 16 for AES-128
    const nonce = randomBytes(12);
    const ciphertext_ = gcm(key, nonce).encrypt(plaintext);
    const plaintext_ = gcm(key, nonce).decrypt(ciphertext_);
    expect(plaintext).toStrictEqual(plaintext_);
  });

  it("SymmetricKey serialization", () => {
    const key = new SymmetricKey();
    console.log(key);

    var serializer = new Serializer();
    key.serialize(serializer);
    const bytes : Uint8Array = serializer.toUint8Array();


    const deserializedKey = SymmetricKey.deserialize(new Deserializer(bytes));

    console.log(bytes);

    expect(deserializedKey).toStrictEqual(key);
  });

  it("SymmetricCiphertext serialization", () => {
    const key = new SymmetricKey();
    const ciphertext = key.encrypt(new Test("hi"));

    var serializer = new Serializer();
    ciphertext.serialize(serializer);
    const bytes : Uint8Array = serializer.toUint8Array();



    const deserializedCiphertext = SymmetricCiphertext.deserialize(new Deserializer(bytes));

    console.log(key);
    console.log(ciphertext);
    console.log(bytes);

    expect(deserializedCiphertext).toStrictEqual(ciphertext);
  });

  it("hmac_kdf consistency with rust", () => {
    console.log(hmac_kdf(Uint8Array.from([1])));
  });

  it("hash to fr", () => {
    console.log(hash_to_fr(Uint8Array.from([1])));
    console.log(hash_to_fr(Uint8Array.from([1,1])));
  });

  it("hash to fq", () => {
    console.log(hash_to_fq(Uint8Array.from([1])));
    console.log(hash_to_fq(Uint8Array.from([1,1])));
  });

});


describe("Curves", () => {
  it("G1 serialization", () => {
    console.log(bn254.G1.Point.BASE.toBytes());
  });

  it("hash_g2_element", () => {
    let g2_bytes = [21, 118, 82, 35, 249, 62, 86, 124, 20, 249, 227, 8, 232, 111, 247, 64, 246, 137, 203, 99, 173, 149, 211, 184, 162, 120, 145, 211, 155, 45, 115, 9, 108, 129, 67, 177, 103, 40, 252, 21, 15, 166, 78, 45, 113, 100, 244, 168, 217, 222, 133, 247, 69, 21, 67, 30, 8, 71, 162, 29, 10, 118, 86, 156];
    let g2 = bytesToG2(new Uint8Array(g2_bytes));
    console.log(g2);
    let hashed_g1 = hash_g2_element(g2);
    console.log(hashed_g1);

  });
});

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Serializable, Serializer, Deserializer } from "@aptos-labs/ts-sdk";
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha2.js';
ed.hashes.sha512 = sha512;
import type { WeierstrassPoint } from '@noble/curves/abstract/weierstrass.js';
import type { Fp2 } from '@noble/curves/abstract/tower.js';
import { get_random_fr, hash_g2_element, hash_to_fr, OneTimePad, SymmetricCiphertext, SymmetricKey } from './symmetric.ts';
import { leBytesToBigint, bigintToLEBytesFr, fp12ToLEBytes } from './fieldSerialization.ts';
import { bytesToG2, G2_SIZE, g2ToBytes } from './curveSerialization.ts';
import { bls12_381 } from '@noble/curves/bls12-381.js';


/**
 * Corresponds to the rust type `aptos_batch_encryption::shared::ciphertext::BIBECiphertext`.
 */
export class BIBECiphertext extends Serializable {
  id: bigint;
  ct_g2: WeierstrassPoint<Fp2>[];
  padded_key: SymmetricKey;
  symmetric_ciphertext: SymmetricCiphertext;

  constructor(id: bigint, ct_g2: WeierstrassPoint<Fp2>[], padded_key: SymmetricKey, symmetric_ciphertext: SymmetricCiphertext) {
    super();
    if (ct_g2.length != 3) {
      throw new Error("Need 3 G2 points here");
    }
    this.id = id;
    this.ct_g2 = ct_g2;
    this.padded_key = padded_key;
    this.symmetric_ciphertext = symmetric_ciphertext;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(bigintToLEBytesFr(this.id));
    // The array of G2 elements is serialized "all at once", i.e., in bcs a single length is given for the whole array. 
    // This has to do w/ the way the arkworks-serde serialization wrapper works in rust.
    let ct_g2_bytes = new Uint8Array(G2_SIZE*3);
    ct_g2_bytes.set(g2ToBytes(this.ct_g2[0]), 0);
    ct_g2_bytes.set(g2ToBytes(this.ct_g2[1]), G2_SIZE*1);
    ct_g2_bytes.set(g2ToBytes(this.ct_g2[2]), G2_SIZE*2);
    serializer.serializeBytes(ct_g2_bytes);
    this.padded_key.serialize(serializer);
    this.symmetric_ciphertext.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): BIBECiphertext {
    const id = leBytesToBigint(deserializer.deserializeBytes());
    const ct_g2_bytes = deserializer.deserializeBytes();
    const ct_g2 = [
      bytesToG2(ct_g2_bytes.slice(0, G2_SIZE)),
      bytesToG2(ct_g2_bytes.slice(G2_SIZE, G2_SIZE*2)),
      bytesToG2(ct_g2_bytes.slice(G2_SIZE*2, G2_SIZE*3)),
    ];
    const padded_key = SymmetricKey.deserialize(deserializer);
    const symmetric_ciphertext = SymmetricCiphertext.deserialize(deserializer);
    return new BIBECiphertext(id, ct_g2, padded_key, symmetric_ciphertext);
  }

}


/**
 * Corresponds to the rust type `aptos_batch_encryption::shared::ciphertext::Ciphertext`.
 */
export class Ciphertext extends Serializable {
  vk: Uint8Array;
  bibe_ct: BIBECiphertext;
  assocated_data_bytes: Uint8Array;
  signature: Uint8Array;

  constructor(vk: Uint8Array, bibe_ct: BIBECiphertext, assocated_data_bytes: Uint8Array, signature: Uint8Array) {
    super();
    this.vk = vk;
    this.bibe_ct = bibe_ct;
    this.assocated_data_bytes = assocated_data_bytes;
    this.signature = signature;
  }

  serialize(serializer: Serializer): void {
    // For some reason, on the rust side, ed25519 VKs are serialized as variable bytes, even though they don't need to be.
    serializer.serializeBytes(this.vk);
    this.bibe_ct.serialize(serializer);
    serializer.serializeBytes(this.assocated_data_bytes);
    // Signatures, however, are serialized as fixed bytes on the rust side.
    serializer.serializeFixedBytes(this.signature);
  }

  static deserialize(deserializer: Deserializer): Ciphertext {
    const vk = deserializer.deserializeBytes();
    const bibe_ct = BIBECiphertext.deserialize(deserializer);
    const associated_data_bytes = deserializer.deserializeBytes();
    const signature = deserializer.deserializeFixedBytes(64);
    return new Ciphertext(vk, bibe_ct, associated_data_bytes, signature);
  }
}

/**
 * Corresponds to the rust type `aptos_batch_encryption::shared::encryption_key::EncryptionKey`.
 */
export class EncryptionKey extends Serializable {
  sig_mpk_g2: WeierstrassPoint<Fp2>;
  tau_g2: WeierstrassPoint<Fp2>;

  constructor(sig_mpk_g2: WeierstrassPoint<Fp2>, tau_g2: WeierstrassPoint<Fp2>) {
    super();
    this.sig_mpk_g2 = sig_mpk_g2;
    this.tau_g2 = tau_g2;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(g2ToBytes(this.sig_mpk_g2));
    serializer.serializeBytes(g2ToBytes(this.tau_g2));
  }

  static deserialize(deserializer: Deserializer): EncryptionKey {
    const sig_mpk_g2 = bytesToG2(deserializer.deserializeBytes());
    const tau_g2 = bytesToG2(deserializer.deserializeBytes());
    return new EncryptionKey(sig_mpk_g2, tau_g2);
  }

  bibe_encrypt(plaintext: Serializable, id: bigint): BIBECiphertext {
    const G2 = bls12_381.G2.Point;
    const Gt = bls12_381.fields.Fp12;


    let r = [get_random_fr(), get_random_fr()];
    let hashed_encryption_key = hash_g2_element(this.sig_mpk_g2);

    let ct_g2 = [
      G2.BASE.multiply(r[0]).add(this.sig_mpk_g2.multiply(r[1])),
      G2.BASE.multiply(id).subtract(this.tau_g2).multiply(r[0]),
      G2.BASE.negate().multiply(r[1]),
    ];


    // Note: in contrast to arkworks, the target group operations are multiplications, not additions.
    // The multiplication by `r[1]` is done inside the pairing because I'm not sure what the interface 
    // is for scalar multiplication over the target group.
    let otp_source_gt = Gt.inv(bls12_381.pairing(hashed_encryption_key.multiply(r[1]), this.sig_mpk_g2));

    let otp_source_bytes = fp12ToLEBytes(otp_source_gt);
    let otp = OneTimePad.from_source_bytes(otp_source_bytes);

    let symmetric_key = new SymmetricKey();
    let padded_key = otp.pad_key(symmetric_key);

    let symmetric_ciphertext = symmetric_key.encrypt(plaintext);

    return new BIBECiphertext(id, ct_g2, padded_key, symmetric_ciphertext);
  }



  encrypt(plaintext: Serializable, associated_data: Serializable): Ciphertext {
    const { secretKey, publicKey } = ed.keygen();
    const hashed_id = hash_to_fr(publicKey);

    const bibe_ct = this.bibe_encrypt(plaintext, hashed_id);

    let associated_data_bytes;
    {
      let serializer = new Serializer();
      associated_data.serialize(serializer);
      associated_data_bytes = serializer.toUint8Array();
    }
    let to_sign;
    {
      let serializer = new Serializer();
      bibe_ct.serialize(serializer);
      serializer.serializeBytes(associated_data_bytes);
      to_sign = serializer.toUint8Array();
    }
    const signature = ed.sign(to_sign, secretKey);

    return new Ciphertext(
      publicKey, 
      bibe_ct,
      associated_data_bytes,
      signature,
    );
  }
}



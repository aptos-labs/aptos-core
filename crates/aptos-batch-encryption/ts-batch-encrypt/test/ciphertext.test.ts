// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { describe, it } from 'vitest';
import { BIBEEncryptionKey } from '../src/ciphertext.js';
import { bn254 } from '@noble/curves/bn254.js';
import { Test } from '../src/symmetric.js';


describe("BIBE ciphertext", () => {

  it("bibe_encrypt", () => {
    let ek = new EncryptionKey(bn254.G2.Point.BASE, bn254.G2.Point.BASE);
    let bibe_ct = ek.bibe_encrypt(new Test("hi"), 1n);
  });
});

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { describe, expect, it} from "vitest";
import { randomBytes } from '@noble/ciphers/utils.js';
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

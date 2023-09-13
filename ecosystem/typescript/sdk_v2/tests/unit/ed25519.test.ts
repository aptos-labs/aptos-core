// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Hex } from "../../src/core/hex";
import { Ed25519PublicKey, Ed25519Signature } from "../../src/crypto/ed25519";

describe("Ed25519", () => {
  it("public key serializes to bytes correctly", async () => {
    const publicKey = "b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200";
    const ed25519PublicKey = new Ed25519PublicKey(publicKey);

    expect(Hex.fromHexInput({ hexInput: ed25519PublicKey.toUint8Array() }).toStringWithoutPrefix()).toEqual(
      "b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200",
    );
  });

  // TODO: Add test for deserializing
});

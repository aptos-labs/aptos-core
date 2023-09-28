// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Serializer } from "../../src/bcs/serializer";
import { Hex } from "../../src/core/hex";
import { Ed25519PublicKey, Ed25519Signature } from "../../src/crypto/ed25519";
import { MultiEd25519PublicKey, MultiEd25519Signature } from "../../src/crypto/multi_ed25519";
import { multiEd25519PkTestObject, multiEd25519SigTestObject } from "./helper";

describe("MultiPublicKey", () => {
  it("should convert to Uint8Array correctly", async () => {
    const publicKey1 = "b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a49200";
    const publicKey2 = "aef3f4a4b8eca1dfc343361bf8e436bd42de9259c04b8314eb8e2054dd6e82ab";
    const publicKey3 = "8a5762e21ac1cdb3870442c77b4c3af58c7cedb8779d0270e6d4f1e2f7367d74";

    const multiPubKey = new MultiEd25519PublicKey({
      publicKeys: [
        new Ed25519PublicKey({ hexInput: publicKey1 }),
        new Ed25519PublicKey({ hexInput: publicKey2 }),
        new Ed25519PublicKey({ hexInput: publicKey3 }),
      ],
      threshold: 2,
    });

    const expected = new Uint8Array([
      185, 198, 238, 22, 48, 239, 62, 113, 17, 68, 166, 72, 219, 6, 187, 178, 40, 79, 114, 116, 207, 190, 229, 63, 252,
      238, 80, 60, 193, 164, 146, 0, 174, 243, 244, 164, 184, 236, 161, 223, 195, 67, 54, 27, 248, 228, 54, 189, 66,
      222, 146, 89, 192, 75, 131, 20, 235, 142, 32, 84, 221, 110, 130, 171, 138, 87, 98, 226, 26, 193, 205, 179, 135, 4,
      66, 199, 123, 76, 58, 245, 140, 124, 237, 184, 119, 157, 2, 112, 230, 212, 241, 226, 247, 54, 125, 116, 2,
    ]);
    expect(multiPubKey.toUint8Array()).toEqual(expected);
  });

  it("should serializes to bytes correctly", async () => {
    let edPksArray = [];
    for (let i = 0; i < multiEd25519PkTestObject.public_keys.length; i++) {
      edPksArray.push(new Ed25519PublicKey({ hexInput: multiEd25519PkTestObject.public_keys[i] }));
    }

    const pubKeyMultiSig = new MultiEd25519PublicKey({
      publicKeys: edPksArray,
      threshold: multiEd25519PkTestObject.threshold,
    });

    expect(Hex.fromHexInput({ hexInput: pubKeyMultiSig.toUint8Array() }).toStringWithoutPrefix()).toEqual(
      multiEd25519PkTestObject.bytesInStringWithoutPrefix,
    );
  });

  it("should deserializes from bytes correctly", async () => {
    let edPksArray = [];
    for (let i = 0; i < multiEd25519PkTestObject.public_keys.length; i++) {
      edPksArray.push(new Ed25519PublicKey({ hexInput: multiEd25519PkTestObject.public_keys[i] }));
    }

    const pubKeyMultiSig = new MultiEd25519PublicKey({
      publicKeys: edPksArray,
      threshold: multiEd25519PkTestObject.threshold,
    });

    const serializer = new Serializer();
    serializer.serialize(pubKeyMultiSig);
    const deserialzed = MultiEd25519PublicKey.deserialize(new Deserializer(serializer.toUint8Array()));
    expect(new Hex({ data: deserialzed.toUint8Array() })).toEqual(new Hex({ data: pubKeyMultiSig.toUint8Array() }));
  });
});

describe("MultiSignature", () => {
  it("should serializes to bytes correctly", async () => {
    let edSigsArray = [];
    for (let i = 0; i < multiEd25519SigTestObject.signatures.length; i++) {
      edSigsArray.push(
        new Ed25519Signature({ data: Hex.fromString({ str: multiEd25519SigTestObject.signatures[i] }).toUint8Array() }),
      );
    }

    const multisig = new MultiEd25519Signature({
      signatures: edSigsArray,
      bitmap: Hex.fromString({ str: multiEd25519SigTestObject.bitmap }).toUint8Array(),
    });

    expect(Hex.fromHexInput({ hexInput: multisig.toUint8Array() }).toStringWithoutPrefix()).toEqual(
      multiEd25519SigTestObject.bytesInStringWithoutPrefix,
    );
  });

  it("should deserializes from bytes correctly", async () => {
    let edSigsArray = [];
    for (let i = 0; i < multiEd25519SigTestObject.signatures.length; i++) {
      edSigsArray.push(
        new Ed25519Signature({ data: Hex.fromString({ str: multiEd25519SigTestObject.signatures[i] }).toUint8Array() }),
      );
    }

    const multisig = new MultiEd25519Signature({
      signatures: edSigsArray,
      bitmap: Hex.fromString({ str: multiEd25519SigTestObject.bitmap }).toUint8Array(),
    });

    const serializer = new Serializer();
    serializer.serialize(multisig);
    const deserialzed = MultiEd25519Signature.deserialize(new Deserializer(serializer.toUint8Array()));
    expect(Hex.fromHexInput({ hexInput: deserialzed.toUint8Array() })).toEqual(
      Hex.fromHexInput({ hexInput: multisig.toUint8Array() }),
    );
  });

  it("should creates a valid bitmap", () => {
    expect(MultiEd25519Signature.createBitmap({ bits: [0, 2, 31] })).toEqual(
      new Uint8Array([0b10100000, 0b00000000, 0b00000000, 0b00000001]),
    );
  });

  it("should throws exception when creating a bitmap with wrong bits", async () => {
    expect(() => {
      MultiEd25519Signature.createBitmap({ bits: [32] });
    }).toThrow("Cannot have a signature larger than 31.");
  });

  it("should throws exception when creating a bitmap with duplicate bits", async () => {
    expect(() => {
      MultiEd25519Signature.createBitmap({ bits: [2, 2] });
    }).toThrow("Duplicate bits detected.");
  });
});

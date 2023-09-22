// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Serializer } from "../../src/bcs/serializer";
import { Hex } from "../../src/core/hex";
import { PublicKey, Signature } from "../../src/crypto/ed25519";
import { MultiPublicKey, MultiSignature } from "../../src/crypto/multi_ed25519";
import { multiEd25519PkTestObject, multiEd25519SigTestObject } from "./helper";

describe("MultiPublicKey", () => {
  it("should serializes to bytes correctly", async () => {
    let edPksArray = [];
    for (let i = 0; i < multiEd25519PkTestObject.public_keys.length; i++) {
      edPksArray.push(new PublicKey({ hexInput: multiEd25519PkTestObject.public_keys[i] }));
    }

    const pubKeyMultiSig = new MultiPublicKey({
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
      edPksArray.push(new PublicKey({ hexInput: multiEd25519PkTestObject.public_keys[i] }));
    }

    const pubKeyMultiSig = new MultiPublicKey({
      publicKeys: edPksArray,
      threshold: multiEd25519PkTestObject.threshold,
    });

    const serializer = new Serializer();
    serializer.serialize(pubKeyMultiSig);
    const deserialzed = MultiPublicKey.deserialize(new Deserializer(serializer.toUint8Array()));
    expect(new Hex({ data: deserialzed.toUint8Array() })).toEqual(new Hex({ data: pubKeyMultiSig.toUint8Array() }));
  });
});

describe("MultiSignature", () => {
  it("should serializes to bytes correctly", async () => {
    let edSigsArray = [];
    for (let i = 0; i < multiEd25519SigTestObject.signatures.length; i++) {
      edSigsArray.push(
        new Signature({ data: Hex.fromString({ str: multiEd25519SigTestObject.signatures[i] }).toUint8Array() }),
      );
    }

    const multisig = new MultiSignature({
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
        new Signature({ data: Hex.fromString({ str: multiEd25519SigTestObject.signatures[i] }).toUint8Array() }),
      );
    }

    const multisig = new MultiSignature({
      signatures: edSigsArray,
      bitmap: Hex.fromString({ str: multiEd25519SigTestObject.bitmap }).toUint8Array(),
    });

    const serializer = new Serializer();
    serializer.serialize(multisig);
    const deserialzed = MultiSignature.deserialize(new Deserializer(serializer.toUint8Array()));
    expect(Hex.fromHexInput({ hexInput: deserialzed.toUint8Array() })).toEqual(
      Hex.fromHexInput({ hexInput: multisig.toUint8Array() }),
    );
  });

  it("should creates a valid bitmap", () => {
    expect(MultiSignature.createBitmap([0, 2, 31])).toEqual(
      new Uint8Array([0b10100000, 0b00000000, 0b00000000, 0b00000001]),
    );
  });

  it("should throws exception when creating a bitmap with wrong bits", async () => {
    expect(() => {
      MultiSignature.createBitmap([32]);
    }).toThrow("Invalid bit value 32.");
  });

  it("should throws exception when creating a bitmap with duplicate bits", async () => {
    expect(() => {
      MultiSignature.createBitmap([2, 2]);
    }).toThrow("Duplicated bits detected.");
  });
});

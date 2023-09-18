// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Serializer } from "../../src/bcs/serializer";
import { Hex } from "../../src/core/hex";
import { Ed25519PublicKey, Ed25519Signature } from "../../src/crypto/ed25519";
import { MultiEd25519PublicKey, MultiEd25519Signature } from "../../src/crypto/multi_ed25519";
import { multiEd25519PkTestObject, multiEd25519SigTestObject } from "./helper";

describe("MultiEd25519", () => {
  it("public key serializes to bytes correctly", async () => {
    let edPksArray = [];
    for (let i = 0; i < multiEd25519PkTestObject.public_keys.length; i++) {
      edPksArray.push(new Ed25519PublicKey(multiEd25519PkTestObject.public_keys[i]));
    }

    const pubKeyMultiSig = new MultiEd25519PublicKey({ publieKeys: edPksArray, threshold: multiEd25519PkTestObject.threshold });

    expect(Hex.fromHexInput({ hexInput: pubKeyMultiSig.toUint8Array() }).toStringWithoutPrefix()).toEqual(
      multiEd25519PkTestObject.bytesInStringWithoutPrefix,
    );
  });

  it("public key deserializes from bytes correctly", async () => {
    let edPksArray = [];
    for (let i = 0; i < multiEd25519PkTestObject.public_keys.length; i++) {
      edPksArray.push(new Ed25519PublicKey(multiEd25519PkTestObject.public_keys[i]));
    }

    const pubKeyMultiSig = new MultiEd25519PublicKey({ publieKeys: edPksArray, threshold: multiEd25519PkTestObject.threshold });

    const serializer = new Serializer();
    serializer.serialize(pubKeyMultiSig);
    const deserialzed = MultiEd25519PublicKey.deserialize(new Deserializer(serializer.toUint8Array()));
    expect(new Hex({ data: deserialzed.toUint8Array() })).toEqual(new Hex({ data: pubKeyMultiSig.toUint8Array() }));
  });

  it("signature serializes to bytes correctly", async () => {
    let edSigsArray = [];
    for (let i = 0; i < multiEd25519SigTestObject.signatures.length; i++) {
      edSigsArray.push(
        new Ed25519Signature(Hex.fromString({ str: multiEd25519SigTestObject.signatures[i] }).toUint8Array()),
      );
    }

    const multisig = new MultiEd25519Signature(
      edSigsArray,
      Hex.fromString({ str: multiEd25519SigTestObject.bitmap }).toUint8Array(),
    );

    expect(Hex.fromHexInput({ hexInput: multisig.toUint8Array() }).toStringWithoutPrefix()).toEqual(
      multiEd25519SigTestObject.bytesInStringWithoutPrefix,
    );
  });

  it("signature deserializes from bytes correctly", async () => {
    let edSigsArray = [];
    for (let i = 0; i < multiEd25519SigTestObject.signatures.length; i++) {
      edSigsArray.push(
        new Ed25519Signature(Hex.fromString({ str: multiEd25519SigTestObject.signatures[i] }).toUint8Array()),
      );
    }

    const multisig = new MultiEd25519Signature(
      edSigsArray,
      Hex.fromString({ str: multiEd25519SigTestObject.bitmap }).toUint8Array(),
    );

    const serializer = new Serializer();
    serializer.serialize(multisig);
    const deserialzed = MultiEd25519Signature.deserialize(new Deserializer(serializer.toUint8Array()));
    expect(Hex.fromHexInput({ hexInput: deserialzed.toUint8Array() })).toEqual(
      Hex.fromHexInput({ hexInput: multisig.toUint8Array() }),
    );
  });

  it("creates a valid bitmap", () => {
    expect(MultiEd25519Signature.createBitmap([0, 2, 31])).toEqual(
      new Uint8Array([0b10100000, 0b00000000, 0b00000000, 0b00000001]),
    );
  });

  it("throws exception when creating a bitmap with wrong bits", async () => {
    expect(() => {
      MultiEd25519Signature.createBitmap([32]);
    }).toThrow("Invalid bit value 32.");
  });

  it("throws exception when creating a bitmap with duplicate bits", async () => {
    expect(() => {
      MultiEd25519Signature.createBitmap([2, 2]);
    }).toThrow("Duplicated bits detected.");
  });
});

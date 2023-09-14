// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Serializer } from "../../src/bcs/serializer";
import { Hex } from "../../src/core/hex";
import { Ed25519PublicKey, Ed25519Signature } from "../../src/crypto/ed25519";

describe("Ed25519PublicKey", () => {
  it("should create instance correctly without error", () => {
    const hexInput = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const publicKey = new Ed25519PublicKey(hexInput);
    expect(publicKey).toBeInstanceOf(Ed25519PublicKey);
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new Ed25519PublicKey(invalidHexInput)).toThrowError(
      `Ed25519PublicKey length should be ${Ed25519PublicKey.LENGTH}`,
    );
  });

  it("should serialize and deserialize correctly", () => {
    const hexInput = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const publicKey = new Ed25519PublicKey(hexInput);
    const serializer = new Serializer();
    publicKey.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedPublicKey = Ed25519PublicKey.deserialize(deserializer);

    expect(deserializedPublicKey).toEqual(publicKey);
  });
});

describe("Ed25519Signature", () => {
  it("should create an instance correctly without error", () => {
    const signatureValue = new Uint8Array(Ed25519Signature.LENGTH);
    const signature = new Ed25519Signature(signatureValue);
    expect(signature).toBeInstanceOf(Ed25519Signature);
  });

  it("should throw an error with invalid value length", () => {
    const invalidSignatureValue = new Uint8Array(Ed25519Signature.LENGTH - 1); // Invalid length
    expect(() => new Ed25519Signature(invalidSignatureValue)).toThrowError(
      `Ed25519Signature length should be ${Ed25519Signature.LENGTH}`,
    );
  });

  it("should serialize and deserialize correctly", () => {
    const signatureValue = new Uint8Array(Ed25519Signature.LENGTH);
    // Initialize the signatureValue with some data if needed
    const signature = new Ed25519Signature(signatureValue);
    const serializer = new Serializer();
    signature.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedSignature = Ed25519Signature.deserialize(deserializer);

    expect(deserializedSignature).toEqual(signature);
  });
});

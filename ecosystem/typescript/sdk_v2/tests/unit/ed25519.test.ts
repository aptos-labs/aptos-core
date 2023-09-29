// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Serializer } from "../../src/bcs/serializer";
import { Hex } from "../../src/core/hex";
import { Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature } from "../../src/crypto/ed25519";
import { ed25519 } from "./helper";

describe("Ed25519PublicKey", () => {
  it("should create the instance correctly without error", () => {
    // Create from string
    const hexStr = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const publicKey = new Ed25519PublicKey({ hexInput: hexStr });
    expect(publicKey).toBeInstanceOf(Ed25519PublicKey);
    expect(publicKey.toString()).toEqual(hexStr);

    // Create from Uint8Array
    const hexUint8Array = new Uint8Array([
      1, 35, 69, 103, 137, 171, 205, 239, 1, 35, 69, 103, 137, 171, 205, 239, 1, 35, 69, 103, 137, 171, 205, 239, 1, 35,
      69, 103, 137, 171, 205, 239,
    ]);
    const publicKey2 = new Ed25519PublicKey({ hexInput: hexUint8Array });
    expect(publicKey2).toBeInstanceOf(Ed25519PublicKey);
    expect(publicKey2.toUint8Array()).toEqual(hexUint8Array);
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new Ed25519PublicKey({ hexInput: invalidHexInput })).toThrowError(
      `PublicKey length should be ${Ed25519PublicKey.LENGTH}`,
    );
  });

  it("should verify the signature correctly", () => {
    const pubKey = new Ed25519PublicKey({ hexInput: ed25519.publicKey });
    const signature = new Ed25519Signature({ hexInput: ed25519.signedMessage });

    // Verify with correct signed message
    expect(pubKey.verifySignature({ message: ed25519.message, signature })).toBe(true);

    // Verify with incorrect signed message
    const incorrectSignedMessage =
      "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0a";
    const invalidSignature = new Ed25519Signature({ hexInput: incorrectSignedMessage });
    expect(pubKey.verifySignature({ message: ed25519.message, signature: invalidSignature })).toBe(false);
  });

  it("should serialize correctly", () => {
    const publicKey = new Ed25519PublicKey({ hexInput: ed25519.publicKey });
    const serializer = new Serializer();
    publicKey.serialize(serializer);

    const expectedUint8Array = new Uint8Array([
      32, 222, 25, 229, 209, 136, 12, 172, 135, 213, 116, 132, 206, 158, 210, 232, 76, 240, 249, 89, 159, 18, 231, 204,
      58, 82, 228, 231, 101, 122, 118, 63, 44,
    ]);
    expect(serializer.toUint8Array()).toEqual(expectedUint8Array);
  });

  it("should deserialize correctly", () => {
    const serializedPublicKey = new Uint8Array([
      32, 222, 25, 229, 209, 136, 12, 172, 135, 213, 116, 132, 206, 158, 210, 232, 76, 240, 249, 89, 159, 18, 231, 204,
      58, 82, 228, 231, 101, 122, 118, 63, 44,
    ]);
    const deserializer = new Deserializer(serializedPublicKey);
    const publicKey = Ed25519PublicKey.deserialize(deserializer);

    expect(publicKey.toString()).toEqual(ed25519.publicKey);
  });

  it("should serialize and deserialize correctly", () => {
    const hexInput = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const publicKey = new Ed25519PublicKey({ hexInput });
    const serializer = new Serializer();
    publicKey.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedPublicKey = Ed25519PublicKey.deserialize(deserializer);

    expect(deserializedPublicKey).toEqual(publicKey);
  });
});

describe("PrivateKey", () => {
  it("should create the instance correctly without error", () => {
    // Create from string
    const privateKey = new Ed25519PrivateKey({ hexInput: ed25519.privateKey });
    expect(privateKey).toBeInstanceOf(Ed25519PrivateKey);
    expect(privateKey.toString()).toEqual(ed25519.privateKey);

    // Create from Uint8Array
    const hexUint8Array = new Uint8Array([
      197, 51, 140, 210, 81, 194, 45, 170, 140, 156, 156, 201, 79, 73, 140, 200, 165, 199, 225, 210, 231, 82, 135, 165,
      221, 169, 16, 150, 254, 100, 239, 165,
    ]);
    const privateKey2 = new Ed25519PrivateKey({ hexInput: hexUint8Array });
    expect(privateKey2).toBeInstanceOf(Ed25519PrivateKey);
    expect(privateKey2.toString()).toEqual(Hex.fromHexInput({ hexInput: hexUint8Array }).toString());
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new Ed25519PrivateKey({ hexInput: invalidHexInput })).toThrowError(
      `PrivateKey length should be ${Ed25519PrivateKey.LENGTH}`,
    );
  });

  it("should sign the message correctly", () => {
    const privateKey = new Ed25519PrivateKey({ hexInput: ed25519.privateKey });
    const signedMessage = privateKey.sign({ message: ed25519.message });
    expect(signedMessage.toString()).toEqual(ed25519.signedMessage);
  });

  it("should serialize correctly", () => {
    const privateKey = new Ed25519PrivateKey({ hexInput: ed25519.privateKey });
    const serializer = new Serializer();
    privateKey.serialize(serializer);

    const expectedUint8Array = new Uint8Array([
      32, 197, 51, 140, 210, 81, 194, 45, 170, 140, 156, 156, 201, 79, 73, 140, 200, 165, 199, 225, 210, 231, 82, 135,
      165, 221, 169, 16, 150, 254, 100, 239, 165,
    ]);
    expect(serializer.toUint8Array()).toEqual(expectedUint8Array);
  });

  it("should deserialize correctly", () => {
    const serializedPrivateKey = new Uint8Array([
      32, 197, 51, 140, 210, 81, 194, 45, 170, 140, 156, 156, 201, 79, 73, 140, 200, 165, 199, 225, 210, 231, 82, 135,
      165, 221, 169, 16, 150, 254, 100, 239, 165,
    ]);
    const deserializer = new Deserializer(serializedPrivateKey);
    const privateKey = Ed25519PrivateKey.deserialize(deserializer);

    expect(privateKey.toString()).toEqual(ed25519.privateKey);
  });

  it("should serialize and deserialize correctly", () => {
    const privateKey = new Ed25519PrivateKey({ hexInput: ed25519.privateKey });
    const serializer = new Serializer();
    privateKey.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedPrivateKey = Ed25519PrivateKey.deserialize(deserializer);

    expect(deserializedPrivateKey.toString()).toEqual(privateKey.toString());
  });

  it("should generate a random private key correctly", () => {
    // Make sure it generate new PrivateKey successfully
    const privateKey = Ed25519PrivateKey.generate();
    expect(privateKey).toBeInstanceOf(Ed25519PrivateKey);
    expect(privateKey.toUint8Array().length).toEqual(Ed25519PrivateKey.LENGTH);

    // Make sure it generate different private keys
    const anotherPrivateKey = Ed25519PrivateKey.generate();
    expect(anotherPrivateKey.toString()).not.toEqual(privateKey.toString());
  });
});

describe("Signature", () => {
  it("should create an instance correctly without error", () => {
    // Create from string
    const signatureStr = new Ed25519Signature({ hexInput: ed25519.signedMessage });
    expect(signatureStr).toBeInstanceOf(Ed25519Signature);
    expect(signatureStr.toString()).toEqual(ed25519.signedMessage);

    // Create from Uint8Array
    const signatureValue = new Uint8Array(Ed25519Signature.LENGTH);
    const signature = new Ed25519Signature({ hexInput: signatureValue });
    expect(signature).toBeInstanceOf(Ed25519Signature);
    expect(signature.toUint8Array()).toEqual(signatureValue);
  });

  it("should throw an error with invalid value length", () => {
    const invalidSignatureValue = new Uint8Array(Ed25519Signature.LENGTH - 1); // Invalid length
    expect(() => new Ed25519Signature({ hexInput: invalidSignatureValue })).toThrowError(
      `Signature length should be ${Ed25519Signature.LENGTH}`,
    );
  });

  it("should serialize correctly", () => {
    const signature = new Ed25519Signature({ hexInput: ed25519.signedMessage });
    const serializer = new Serializer();
    signature.serialize(serializer);

    const expectedUint8Array = new Uint8Array([
      64, 197, 222, 158, 64, 172, 0, 179, 113, 205, 131, 177, 193, 151, 250, 91, 102, 91, 116, 73, 179, 60, 211, 205,
      211, 5, 187, 120, 34, 46, 6, 166, 113, 164, 150, 37, 171, 154, 234, 138, 3, 157, 75, 183, 14, 39, 87, 104, 8, 77,
      98, 176, 148, 188, 27, 49, 150, 79, 35, 87, 183, 193, 175, 126, 13,
    ]);
    expect(serializer.toUint8Array()).toEqual(expectedUint8Array);
  });

  it("should deserialize correctly", () => {
    const serializedSignature = new Uint8Array([
      64, 197, 222, 158, 64, 172, 0, 179, 113, 205, 131, 177, 193, 151, 250, 91, 102, 91, 116, 73, 179, 60, 211, 205,
      211, 5, 187, 120, 34, 46, 6, 166, 113, 164, 150, 37, 171, 154, 234, 138, 3, 157, 75, 183, 14, 39, 87, 104, 8, 77,
      98, 176, 148, 188, 27, 49, 150, 79, 35, 87, 183, 193, 175, 126, 13,
    ]);
    const deserializer = new Deserializer(serializedSignature);
    const signature = Ed25519Signature.deserialize(deserializer);

    expect(signature.toString()).toEqual(ed25519.signedMessage);
  });

  it("should serialize and deserialize correctly", () => {
    const signatureValue = new Uint8Array(Ed25519Signature.LENGTH);
    const signature = new Ed25519Signature({ hexInput: signatureValue });
    const serializer = new Serializer();
    signature.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedSignature = Ed25519Signature.deserialize(deserializer);

    expect(deserializedSignature.toUint8Array()).toEqual(signature.toUint8Array());
  });
});

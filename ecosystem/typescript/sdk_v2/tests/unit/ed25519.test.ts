// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Serializer } from "../../src/bcs/serializer";
import { Hex } from "../../src/core/hex";
import { PrivateKey, PublicKey, Signature } from "../../src/crypto/ed25519";
import { ed25519 } from "./helper";

describe("PublicKey", () => {
  it("should create the instance correctly without error", () => {
    // Create from string
    const hexStr = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const publicKey = new PublicKey({ hexInput: hexStr });
    expect(publicKey).toBeInstanceOf(PublicKey);
    expect(publicKey.toString()).toEqual(hexStr);

    // Create from Uint8Array
    const hexUint8Array = new Uint8Array(PublicKey.LENGTH);
    const publicKey2 = new PublicKey({ hexInput: hexUint8Array });
    expect(publicKey2).toBeInstanceOf(PublicKey);
    expect(publicKey2.toString()).toEqual(hexStr);
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new PublicKey({ hexInput: invalidHexInput })).toThrowError(
      `PublicKey length should be ${PublicKey.LENGTH}`,
    );
  });

  it("should verify the signature correctly", () => {
    const pubKey = new PublicKey({ hexInput: ed25519.publicKey });

    // Verify with correct signed message
    expect(pubKey.verifySignature({ data: ed25519.message, signature: ed25519.signedMessage })).toBe(true);

    // Verify with incorrect signed message
    const incorrectSignedMessage =
      "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0a";
    expect(pubKey.verifySignature({ data: ed25519.message, signature: incorrectSignedMessage })).toBe(false);
  });

  it("should serialize and deserialize correctly", () => {
    const hexInput = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const publicKey = new PublicKey({ hexInput });
    const serializer = new Serializer();
    publicKey.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedPublicKey = PublicKey.deserialize(deserializer);

    expect(deserializedPublicKey).toEqual(publicKey);
  });
});

describe("PrivateKey", () => {
  it("should create the instance correctly without error", () => {
    // Create from string
    const privateKey = new PrivateKey({ value: ed25519.privateKey });
    expect(privateKey).toBeInstanceOf(PrivateKey);
    expect(privateKey.toString()).toEqual(ed25519.privateKey);

    // Create from Uint8Array
    const hexUint8Array = new Uint8Array(PrivateKey.LENGTH);
    const privateKey2 = new PrivateKey({ value: hexUint8Array });
    expect(privateKey2).toBeInstanceOf(PrivateKey);
    expect(privateKey2.toString()).toEqual(Hex.fromHexInput({ hexInput: hexUint8Array }).toString());
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new PrivateKey({ value: invalidHexInput })).toThrowError(
      `PrivateKey length should be ${PrivateKey.LENGTH}`,
    );
  });

  it("should sign the message correctly", () => {
    const privateKey = new PrivateKey({ value: ed25519.privateKey });
    const signedMessage = privateKey.sign({ message: ed25519.message });
    expect(signedMessage.toString()).toEqual(ed25519.signedMessage);
  });

  it("should serialize and deserialize correctly", () => {
    const privateKey = new PrivateKey({ value: ed25519.privateKey });
    const serializer = new Serializer();
    privateKey.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedPrivateKey = PrivateKey.deserialize(deserializer);

    expect(deserializedPrivateKey.toString()).toEqual(privateKey.toString());
  });

  it("should generate a random private key correctly", () => {
    // Make sure it generate new PrivateKey successfully
    const privateKey = PrivateKey.generate();
    expect(privateKey).toBeInstanceOf(PrivateKey);
    expect(privateKey.toUint8Array().length).toEqual(PrivateKey.LENGTH);

    // Make sure it generate different private keys
    const anotherPrivateKey = PrivateKey.generate();
    expect(anotherPrivateKey.toString()).not.toEqual(privateKey.toString());
  });
});

describe("Signature", () => {
  it("should create an instance correctly without error", () => {
    // Create from string
    const signatureStr = new Signature({ data: ed25519.signedMessage });
    expect(signatureStr).toBeInstanceOf(Signature);
    expect(signatureStr.toString()).toEqual(ed25519.signedMessage);

    // Create from Uint8Array
    const signatureValue = new Uint8Array(Signature.LENGTH);
    const signature = new Signature({ data: signatureValue });
    expect(signature).toBeInstanceOf(Signature);
    expect(signature.toUint8Array()).toEqual(signatureValue);
  });

  it("should throw an error with invalid value length", () => {
    const invalidSignatureValue = new Uint8Array(Signature.LENGTH - 1); // Invalid length
    expect(() => new Signature({ data: invalidSignatureValue })).toThrowError(
      `Signature length should be ${Signature.LENGTH}`,
    );
  });

  it("should serialize and deserialize correctly", () => {
    const signatureValue = new Uint8Array(Signature.LENGTH);
    const signature = new Signature({ data: signatureValue });
    const serializer = new Serializer();
    signature.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedSignature = Signature.deserialize(deserializer);

    expect(deserializedSignature.toUint8Array()).toEqual(signature.toUint8Array());
  });
});

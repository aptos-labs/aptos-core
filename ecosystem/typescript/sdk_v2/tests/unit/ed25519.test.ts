// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Deserializer } from "../../src/bcs/deserializer";
import { Serializer } from "../../src/bcs/serializer";
import { PrivateKey, PublicKey, Signature } from "../../src/crypto/ed25519";

// eslint-disable-next-line max-len
const mockPrivateKey = "0xc5338cd251c22daa8c9c9cc94f498cc8a5c7e1d2e75287a5dda91096fe64efa5";
const mockPublicKey = "0xde19e5d1880cac87d57484ce9ed2e84cf0f9599f12e7cc3a52e4e7657a763f2c";
const messageHex = "0x7777";
const expectedSignedMessage =
  "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0d";
describe("PublicKey", () => {
  it("should create the instance correctly without error", () => {
    const hexInput = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const publicKey = new PublicKey({ hexInput });
    expect(publicKey).toBeInstanceOf(PublicKey);
    expect(publicKey.toString()).toEqual(hexInput);
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new PublicKey({ hexInput: invalidHexInput })).toThrowError(
      `PublicKey length should be ${PublicKey.LENGTH}`,
    );
  });

  it("should verify the signature correctly", () => {
    const pubKey = new PublicKey({ hexInput: mockPublicKey });

    // Verify with correct signed message
    expect(pubKey.verifySignature({ message: messageHex, signature: expectedSignedMessage })).toBe(true);

    // Verify with incorrect signed message
    const incorrectSignedMessage =
      "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0a";
    expect(pubKey.verifySignature({ message: messageHex, signature: incorrectSignedMessage })).toBe(false);
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
    const privateKey = new PrivateKey({ value: mockPrivateKey });
    expect(privateKey).toBeInstanceOf(PrivateKey);
    expect(privateKey.toString()).toEqual(mockPrivateKey.toString());
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new PrivateKey({ value: invalidHexInput })).toThrowError(
      `PrivateKey length should be ${PrivateKey.LENGTH}`,
    );
  });

  it("should sign the message correctly", () => {
    const privateKey = new PrivateKey({ value: mockPrivateKey });
    const signedMessage = privateKey.sign({ message: messageHex });
    expect(signedMessage.toString()).toEqual(expectedSignedMessage);
  });

  it("should serialize and deserialize correctly", () => {
    const privateKey = new PrivateKey({ value: mockPrivateKey });
    const serializer = new Serializer();
    privateKey.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedPrivateKey = PrivateKey.deserialize(deserializer);

    expect(deserializedPrivateKey.toString()).toEqual(privateKey.toString());
  });
});

describe("Signature", () => {
  it("should create an instance correctly without error", () => {
    const signatureValue = new Uint8Array(Signature.LENGTH);
    const signature = new Signature({ value: signatureValue });
    expect(signature).toBeInstanceOf(Signature);
    expect(signature.toUint8Array()).toEqual(signatureValue);
  });

  it("should throw an error with invalid value length", () => {
    const invalidSignatureValue = new Uint8Array(Signature.LENGTH - 1); // Invalid length
    expect(() => new Signature({ value: invalidSignatureValue })).toThrowError(
      `Signature length should be ${Signature.LENGTH}`,
    );
  });

  it("should serialize and deserialize correctly", () => {
    const signatureValue = new Uint8Array(Signature.LENGTH);
    const signature = new Signature({ value: signatureValue });
    const serializer = new Serializer();
    signature.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedSignature = Signature.deserialize(deserializer);

    expect(deserializedSignature.toUint8Array()).toEqual(signature.toUint8Array());
  });
});

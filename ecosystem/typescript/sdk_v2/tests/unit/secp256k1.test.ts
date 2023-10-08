// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Hex } from "../../src/core/hex";
import { Secp256k1PrivateKey, Secp256k1PublicKey, Secp256k1Signature } from "../../src/crypto/secp256k1";
import { secp256k1 } from "@noble/curves/secp256k1";
import { secp256k1TestObject } from "./helper";
import { Serializer } from "../../src/bcs/serializer";
import { Deserializer } from "../../src/bcs/deserializer";

describe("Secp256k1PublicKey", () => {
  it("should create the instance correctly without error", () => {
    // Create from string
    const publicKey = new Secp256k1PublicKey({ hexInput: secp256k1TestObject.publicKey });
    expect(publicKey).toBeInstanceOf(Secp256k1PublicKey);
    expect(publicKey.toString()).toEqual(secp256k1TestObject.publicKey);

    // // Create from Uint8Array
    const hexUint8Array = secp256k1.getPublicKey(secp256k1.utils.randomPrivateKey(), false);
    const publicKey2 = new Secp256k1PublicKey({ hexInput: hexUint8Array });
    expect(publicKey2).toBeInstanceOf(Secp256k1PublicKey);
    expect(publicKey2.toUint8Array()).toEqual(hexUint8Array);
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new Secp256k1PublicKey({ hexInput: invalidHexInput })).toThrowError(
      `PublicKey length should be ${Secp256k1PublicKey.LENGTH}`,
    );
  });

  it("should verify the signature correctly", () => {
    const pubKey = new Secp256k1PublicKey({ hexInput: secp256k1TestObject.publicKey });
    const signature = new Secp256k1Signature({ hexInput: secp256k1TestObject.signatureHex });

    // Convert message to hex
    const hexMsg = Hex.fromString({ str: secp256k1TestObject.messageEncoded });

    // Verify with correct signed message
    expect(pubKey.verifySignature({ message: hexMsg.toUint8Array(), signature })).toBe(true);

    // Verify with incorrect signed message
    const incorrectSignedMessage =
      "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0a";
    const invalidSignature = new Secp256k1Signature({ hexInput: incorrectSignedMessage });
    expect(pubKey.verifySignature({ message: secp256k1TestObject.messageEncoded, signature: invalidSignature })).toBe(
      false,
    );
  });

  it("should serialize correctly", () => {
    const publicKey = new Secp256k1PublicKey({ hexInput: secp256k1TestObject.publicKey });
    const serializer = new Serializer();
    publicKey.serialize(serializer);

    const serialized = Hex.fromHexInput({ hexInput: serializer.toUint8Array() }).toString();
    const expected =
      "0x4104acdd16651b839c24665b7e2033b55225f384554949fef46c397b5275f37f6ee95554d70fb5d9f93c5831ebf695c7206e7477ce708f03ae9bb2862dc6c9e033ea";
    expect(serialized).toEqual(expected);
  });

  it("should deserialize correctly", () => {
    const serializedPublicKeyStr =
      "0x4104acdd16651b839c24665b7e2033b55225f384554949fef46c397b5275f37f6ee95554d70fb5d9f93c5831ebf695c7206e7477ce708f03ae9bb2862dc6c9e033ea";
    const serializedPublicKey = Hex.fromString({ str: serializedPublicKeyStr }).toUint8Array();
    const deserializer = new Deserializer(serializedPublicKey);
    const publicKey = Secp256k1PublicKey.deserialize(deserializer);

    expect(publicKey.toString()).toEqual(secp256k1TestObject.publicKey);
  });
});

describe("Secp256k1PrivateKey", () => {
  it("should create the instance correctly without error", () => {
    // Create from string
    const privateKey = new Secp256k1PrivateKey({ hexInput: secp256k1TestObject.privateKey });
    expect(privateKey).toBeInstanceOf(Secp256k1PrivateKey);
    expect(privateKey.toString()).toEqual(secp256k1TestObject.privateKey);

    // Create from Uint8Array
    const hexUint8Array = Hex.fromString({ str: secp256k1TestObject.privateKey }).toUint8Array();
    const privateKey2 = new Secp256k1PrivateKey({ hexInput: hexUint8Array });
    expect(privateKey2).toBeInstanceOf(Secp256k1PrivateKey);
    expect(privateKey2.toString()).toEqual(Hex.fromHexInput({ hexInput: hexUint8Array }).toString());
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new Secp256k1PrivateKey({ hexInput: invalidHexInput })).toThrowError(
      `PrivateKey length should be ${Secp256k1PrivateKey.LENGTH}`,
    );
  });

  it("should sign the message correctly", () => {
    const privateKey = new Secp256k1PrivateKey({ hexInput: secp256k1TestObject.privateKey });
    const signedMessage = privateKey.sign({ message: secp256k1TestObject.messageEncoded });
    expect(signedMessage.toString()).toEqual(secp256k1TestObject.signatureHex);
  });

  it("should serialize correctly", () => {
    const privateKey = new Secp256k1PrivateKey({ hexInput: secp256k1TestObject.privateKey });
    const serializer = new Serializer();
    privateKey.serialize(serializer);

    const received = Hex.fromHexInput({ hexInput: serializer.toUint8Array() }).toString();
    const expected = "0x20d107155adf816a0a94c6db3c9489c13ad8a1eda7ada2e558ba3bfa47c020347e";
    expect(received).toEqual(expected);
  });

  it("should deserialize correctly", () => {
    const serializedPrivateKeyStr = "0x20d107155adf816a0a94c6db3c9489c13ad8a1eda7ada2e558ba3bfa47c020347e";
    const serializedPrivateKey = Hex.fromString({ str: serializedPrivateKeyStr }).toUint8Array();
    const deserializer = new Deserializer(serializedPrivateKey);
    const privateKey = Secp256k1PrivateKey.deserialize(deserializer);

    expect(privateKey.toString()).toEqual(secp256k1TestObject.privateKey);
  });

  it("should serialize and deserialize correctly", () => {
    const privateKey = new Secp256k1PrivateKey({ hexInput: secp256k1TestObject.privateKey });
    const serializer = new Serializer();
    privateKey.serialize(serializer);

    const deserializer = new Deserializer(serializer.toUint8Array());
    const deserializedPrivateKey = Secp256k1PrivateKey.deserialize(deserializer);

    expect(deserializedPrivateKey.toString()).toEqual(privateKey.toString());
  });
});

describe("Secp256k1Signature", () => {
  it("should create an instance correctly without error", () => {
    // Create from string
    const signatureStr = new Secp256k1Signature({ hexInput: secp256k1TestObject.signatureHex });
    expect(signatureStr).toBeInstanceOf(Secp256k1Signature);
    expect(signatureStr.toString()).toEqual(secp256k1TestObject.signatureHex);

    // Create from Uint8Array
    const signatureValue = new Uint8Array(Secp256k1Signature.LENGTH);
    const signature = new Secp256k1Signature({ hexInput: signatureValue });
    expect(signature).toBeInstanceOf(Secp256k1Signature);
    expect(signature.toUint8Array()).toEqual(signatureValue);
  });

  it("should throw an error with invalid value length", () => {
    const invalidSignatureValue = new Uint8Array(Secp256k1Signature.LENGTH - 1); // Invalid length
    expect(() => new Secp256k1Signature({ hexInput: invalidSignatureValue })).toThrowError(
      `Signature length should be ${Secp256k1Signature.LENGTH}`,
    );
  });

  it("should serialize correctly", () => {
    const signature = new Secp256k1Signature({ hexInput: secp256k1TestObject.signatureHex });
    const serializer = new Serializer();
    signature.serialize(serializer);

    const received = Hex.fromHexInput({ hexInput: serializer.toUint8Array() }).toString();
    const expected =
      "0x403eda29841168c902b154ac12dfb0f8775ece1b95315b227ede64cbd715abac665aa8c8df5b108b0d4918bb88ea58c892972af375a71761a7e590655ff5de3859";
    expect(received).toEqual(expected);
  });

  it("should deserialize correctly", () => {
    const serializedSignature =
      "0x403eda29841168c902b154ac12dfb0f8775ece1b95315b227ede64cbd715abac665aa8c8df5b108b0d4918bb88ea58c892972af375a71761a7e590655ff5de3859";
    const serializedSignatureUint8Array = Hex.fromString({ str: serializedSignature }).toUint8Array();
    const deserializer = new Deserializer(serializedSignatureUint8Array);
    const signature = Secp256k1Signature.deserialize(deserializer);

    expect(signature.toString()).toEqual(secp256k1TestObject.signatureHex);
  });
});

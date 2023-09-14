// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AuthenticationKey } from "../../src/crypto/authentication_key";
import { Ed25519PublicKey } from "../../src/crypto/ed25519";
import { MultiEd25519PublicKey } from "../../src/crypto/multi_ed25519";

// Auth key and Public key pair for testing
const auth_key_hexInput = "0x6324287105756b0338e0f84025bd0ac80e58154eb94257b0d4f06ec6497e656e";
const public_key = "0x719ab6a6d406931ca80efa922e3377390a8d2803e42ecdbf394e979f9a5e57bc";

describe("AuthenticationKey", () => {
  it("should create an instance with save the hexinput correctly", () => {
    const authKey = new AuthenticationKey(auth_key_hexInput);
    expect(authKey).toBeInstanceOf(AuthenticationKey);
    expect(authKey.data.toString()).toEqual(auth_key_hexInput);
  });

  it("should throw an error with invalid hex input length", () => {
    const invalidHexInput = "0123456789abcdef"; // Invalid length
    expect(() => new AuthenticationKey(invalidHexInput)).toThrowError("Expected a hexinput of length 32");
  });

  it("should create AuthenticationKey from Ed25519PublicKey", () => {
    const publicKey = new Ed25519PublicKey(public_key);
    const authKey = AuthenticationKey.fromEd25519PublicKey(publicKey);
    expect(authKey).toBeInstanceOf(AuthenticationKey);
    expect(authKey.data.toString()).toEqual(auth_key_hexInput);
  });

  it("should create AuthenticationKey from MultiEd25519PublicKey", () => {
    const publicKey = new MultiEd25519PublicKey([new Ed25519PublicKey(public_key)], 1);
    const authKey = AuthenticationKey.fromMultiEd25519PublicKey(publicKey);
    expect(authKey).toBeInstanceOf(AuthenticationKey);
    expect(authKey.data.toString()).toEqual("0xd0cb1ed17413857dd59f4ee948b6678c339ad6d5cc97246f65825439f53fd944");
  });

  it("should derive an AccountAddress from AuthenticationKey with same string", () => {
    const authKey = new AuthenticationKey(auth_key_hexInput);
    const accountAddress = authKey.derivedAddress();
    expect(accountAddress.toString()).toEqual(auth_key_hexInput);
  });
});

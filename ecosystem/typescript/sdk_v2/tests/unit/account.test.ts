// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Account } from "../../src/core/account";
import { AccountAddress } from "../../src/core/account_address";
import { Hex } from "../../src/core/hex";
import { Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature } from "../../src/crypto/ed25519";
import { Secp256k1PrivateKey, Secp256k1PublicKey, Secp256k1Signature } from "../../src/crypto/secp256k1";
import { SigningScheme } from "../../src/types";
import { ed25519, secp256k1TestObject, wallet } from "./helper";

describe("Ed25519 Account", () => {
  it("should create an instance of Account correctly without error", () => {
    // Account with Ed25519 scheme
    const edAccount = Account.generate({ scheme: SigningScheme.Ed25519 });
    expect(edAccount).toBeInstanceOf(Account);
    expect(edAccount.signingScheme).toEqual(SigningScheme.Ed25519);
  });

  it("should create a new account from a provided private key", () => {
    const { privateKey: privateKeyBytes, publicKey, address } = ed25519;
    const privateKey = new Ed25519PrivateKey({ hexInput: privateKeyBytes });
    const newAccount = Account.fromPrivateKey({ privateKey });
    expect(newAccount).toBeInstanceOf(Account);
    expect((newAccount.privateKey as Ed25519PrivateKey).toString()).toEqual(privateKey.toString());
    expect((newAccount.publicKey as Ed25519PublicKey).toString()).toEqual(
      new Ed25519PublicKey({ hexInput: publicKey }).toString(),
    );
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should create a new account from a provided private key and address", () => {
    const { privateKey: privateKeyBytes, publicKey, address } = ed25519;
    const privateKey = new Ed25519PrivateKey({ hexInput: privateKeyBytes });
    const newAccount = Account.fromPrivateKeyAndAddress({
      privateKey,
      address: AccountAddress.fromString({ input: address }),
    });
    expect(newAccount).toBeInstanceOf(Account);
    expect((newAccount.privateKey as Ed25519PrivateKey).toString()).toEqual(privateKey.toString());
    expect((newAccount.publicKey as Ed25519PublicKey).toString()).toEqual(
      new Ed25519PublicKey({ hexInput: publicKey }).toString(),
    );
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should create a new account from a bip44 path and mnemonics", () => {
    const { mnemonic, address, path } = wallet;
    const newAccount = Account.fromDerivationPath({ path, mnemonic });
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should prevent an invalid bip44 path ", () => {
    const { mnemonic } = wallet;
    const path = "1234";
    expect(() => Account.fromDerivationPath({ path, mnemonic })).toThrow("Invalid derivation path");
  });

  it("should check if a derivation path is valid", () => {
    const validPath = wallet.path; // Valid path
    const invalidPath = "invalid/path"; // Invalid path
    expect(Account.isValidPath({ path: validPath })).toBe(true);
    expect(Account.isValidPath({ path: invalidPath })).toBe(false);
  });

  it("should return the authentication key for a public key", () => {
    const { publicKey: publicKeyBytes, address } = ed25519;
    const publicKey = new Ed25519PublicKey({ hexInput: publicKeyBytes });
    const authKey = Account.authKey({ publicKey });
    expect(authKey).toBeInstanceOf(Hex);
    expect(authKey.toString()).toEqual(address);
  });

  it("should sign data, return a Hex signature, and verify", () => {
    const { privateKey: privateKeyBytes, address, message, signedMessage } = ed25519;
    const privateKey = new Ed25519PrivateKey({ hexInput: privateKeyBytes });
    const account = Account.fromPrivateKeyAndAddress({
      privateKey,
      address: AccountAddress.fromString({ input: address }),
    });
    expect(account.sign({ data: message }).toString()).toEqual(signedMessage);

    // Verify the signature
    const signature = new Ed25519Signature({ hexInput: signedMessage });
    expect(account.verifySignature({ message, signature })).toBe(true);
  });
});

describe("Secp256k1 Account", () => {
  it("should create an instance of Account correctly without error", () => {
    // Account with Secp256k1 scheme
    const secp256k1Account = Account.generate({ scheme: SigningScheme.Secp256k1Ecdsa });
    expect(secp256k1Account).toBeInstanceOf(Account);
    expect(secp256k1Account.signingScheme).toEqual(SigningScheme.Secp256k1Ecdsa);
  });

  it("should create a new account from a provided private key", () => {
    const { privateKey: privateKeyBytes, publicKey, address } = secp256k1TestObject;
    const privateKey = new Secp256k1PrivateKey({ hexInput: privateKeyBytes });
    const newAccount = Account.fromPrivateKey({ privateKey });
    expect(newAccount).toBeInstanceOf(Account);
    expect((newAccount.privateKey as Secp256k1PrivateKey).toString()).toEqual(privateKey.toString());
    expect((newAccount.publicKey as Secp256k1PublicKey).toString()).toEqual(
      new Secp256k1PublicKey({ hexInput: publicKey }).toString(),
    );
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should create a new account from a provided private key and address", () => {
    const { privateKey: privateKeyBytes, publicKey, address } = secp256k1TestObject;
    const privateKey = new Secp256k1PrivateKey({ hexInput: privateKeyBytes });
    const newAccount = Account.fromPrivateKeyAndAddress({
      privateKey,
      address: AccountAddress.fromString({ input: address }),
    });
    expect(newAccount).toBeInstanceOf(Account);
    expect((newAccount.privateKey as Secp256k1PrivateKey).toString()).toEqual(privateKey.toString());
    expect((newAccount.publicKey as Secp256k1PublicKey).toString()).toEqual(
      new Secp256k1PublicKey({ hexInput: publicKey }).toString(),
    );
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should create a new account from a bip44 path and mnemonics", () => {
    const { mnemonic, address, path } = wallet;
    const newAccount = Account.fromDerivationPath({ path, mnemonic });
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should prevent an invalid bip44 path ", () => {
    const { mnemonic } = wallet;
    const path = "1234";
    expect(() => Account.fromDerivationPath({ path, mnemonic })).toThrow("Invalid derivation path");
  });

  it("should check if a derivation path is valid", () => {
    const validPath = wallet.path; // Valid path
    const invalidPath = "invalid/path"; // Invalid path
    expect(Account.isValidPath({ path: validPath })).toBe(true);
    expect(Account.isValidPath({ path: invalidPath })).toBe(false);
  });

  it("should return the authentication key for a public key", () => {
    const { publicKey: publicKeyBytes, address } = secp256k1TestObject;
    const publicKey = new Secp256k1PublicKey({ hexInput: publicKeyBytes });
    const authKey = Account.authKey({ publicKey });
    expect(authKey).toBeInstanceOf(Hex);
    expect(authKey.toString()).toEqual(address);
  });

  it("should sign data, return a Hex signature, and verify", () => {
    const { privateKey: privateKeyBytes, address, signatureHex, messageEncoded } = secp256k1TestObject;

    // Sign the message
    const secp256k1PrivateKey = new Secp256k1PrivateKey({ hexInput: privateKeyBytes });
    const account = Account.fromPrivateKeyAndAddress({
      privateKey: secp256k1PrivateKey,
      address: AccountAddress.fromString({ input: address }),
    });
    const signedMessage = account.sign({ data: messageEncoded });
    expect(signedMessage.toString()).toEqual(signatureHex);

    // Verify the signature
    const signature = new Secp256k1Signature({ hexInput: signatureHex });
    expect(account.verifySignature({ message: messageEncoded, signature })).toBe(true);
  });
});

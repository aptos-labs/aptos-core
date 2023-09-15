// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Account } from "../../src/core/account";
import { AccountAddress } from "../../src/core/account_address";
import { Hex } from "../../src/core/hex";

// eslint-disable-next-line max-len
const mockPrivateKey = Hex.fromString({
  str: "0xc5338cd251c22daa8c9c9cc94f498cc8a5c7e1d2e75287a5dda91096fe64efa5de19e5d1880cac87d57484ce9ed2e84cf0f9599f12e7cc3a52e4e7657a763f2c",
});
const mockAddress = AccountAddress.fromString({
  input: "0x978c213990c4833df71548df7ce49d54c759d6b6d932de22b24d56060b7af2aa",
});
const mockPublicKey = Hex.fromString({ str: "0xde19e5d1880cac87d57484ce9ed2e84cf0f9599f12e7cc3a52e4e7657a763f2c" });
const mnemonic = "shoot island position soft burden budget tooth cruel issue economy destroy above";

describe("Account", () => {
  it("should create an instance of Account correctly without error", () => {
    const account = Account.create();
    expect(account).toBeInstanceOf(Account);
  });

  it("should create a new account from a provided private key", () => {
    const newAccount = Account.fromPrivateKey(mockPrivateKey.toString());
    expect(newAccount).toBeInstanceOf(Account);
    expect(newAccount.privateKey.toString()).toEqual(mockPrivateKey.toString());
    expect(newAccount.publicKey.toString()).toEqual(mockPublicKey.toString());
    expect(newAccount.accountAddress.toString()).toEqual(mockAddress.toString());
  });

  it("should create a new account from a provided private key and address", () => {
    const newAccount = Account.fromPrivateKeyAndAddress(mockPrivateKey.toString(), mockAddress);
    expect(newAccount).toBeInstanceOf(Account);
    expect(newAccount.privateKey.toString()).toEqual(mockPrivateKey.toString());
    expect(newAccount.publicKey.toString()).toEqual(mockPublicKey.toString());
    expect(newAccount.accountAddress.toString()).toEqual(mockAddress.toString());
  });

  it("should create a new account from a provided private key and address", () => {
    const newAccount = Account.fromPrivateKeyAndAddress(mockPrivateKey.toString(), mockAddress);
    expect(newAccount).toBeInstanceOf(Account);
    expect(newAccount.privateKey.toString()).toEqual(mockPrivateKey.toString());
  });

  it("should create a new account from a bip44 path and mnemonics", () => {
    const address = "0x07968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30";
    const bip44Path = "m/44'/637'/0'/0'/0'";
    const newAccount = Account.fromDerivationPath(bip44Path, mnemonic);
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should check if a derivation path is valid", () => {
    const validPath = "m/44'/637'/0'/0'/0'"; // Valid path
    const invalidPath = "invalid/path"; // Invalid path
    expect(Account.isValidPath(validPath)).toBe(true);
    expect(Account.isValidPath(invalidPath)).toBe(false);
  });

  it("should return the authentication key for a public key", () => {
    const authKey = Account.authKey(mockPublicKey.toUint8Array());
    expect(authKey).toBeInstanceOf(Hex);
    expect(authKey.toString()).toEqual(mockAddress.toString());
  });

  it("should sign data, return a Hex signature, and verify", () => {
    const account = Account.fromPrivateKeyAndAddress(mockPrivateKey.toString(), mockAddress);
    const messageHex = "0x7777";
    const expectedSignedMessage =
      "0xc5de9e40ac00b371cd83b1c197fa5b665b7449b33cd3cdd305bb78222e06a671a49625ab9aea8a039d4bb70e275768084d62b094bc1b31964f2357b7c1af7e0d";
    expect(account.sign(messageHex).toString()).toEqual(expectedSignedMessage);
    expect(account.verifySignature(messageHex, expectedSignedMessage)).toBe(true);
    expect(account.verifySignature(messageHex, expectedSignedMessage)).toBe(true);
  });
});

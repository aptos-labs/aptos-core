// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Account } from "../../src/core/account";
import { AccountAddress } from "../../src/core/account_address";
import { Hex } from "../../src/core/hex";
import { ed25519 } from "./helper";

describe("Account", () => {
  it("should create an instance of Account correctly without error", () => {
    const account = Account.generate();
    expect(account).toBeInstanceOf(Account);
  });

  it("should create a new account from a provided private key", () => {
    const { privateKey, publicKey, address } = ed25519;
    const newAccount = Account.fromPrivateKey({ privateKey });
    expect(newAccount).toBeInstanceOf(Account);
    expect(newAccount.privateKey.toString()).toEqual(privateKey);
    expect(newAccount.publicKey.toString()).toEqual(publicKey);
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should create a new account from a provided private key and address", () => {
    const { privateKey, publicKey, address } = ed25519;
    const newAccount = Account.fromPrivateKeyAndAddress({
      privateKey,
      address: AccountAddress.fromString({ input: address }),
    });
    expect(newAccount).toBeInstanceOf(Account);
    expect(newAccount.privateKey.toString()).toEqual(privateKey);
    expect(newAccount.publicKey.toString()).toEqual(publicKey);
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should create a new account from a bip44 path and mnemonics", () => {
    const { mnemonic } = ed25519;
    const address = "0x07968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30";
    const path = "m/44'/637'/0'/0'/0'";
    const newAccount = Account.fromDerivationPath({ path, mnemonic });
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should check if a derivation path is valid", () => {
    const validPath = "m/44'/637'/0'/0'/0'"; // Valid path
    const invalidPath = "invalid/path"; // Invalid path
    expect(Account.isValidPath({ path: validPath })).toBe(true);
    expect(Account.isValidPath({ path: invalidPath })).toBe(false);
  });

  it("should return the authentication key for a public key", () => {
    const { publicKey, address } = ed25519;
    const authKey = Account.authKey({ publicKey });
    expect(authKey).toBeInstanceOf(Hex);
    expect(authKey.toString()).toEqual(address);
  });

  it("should sign data, return a Hex signature, and verify", () => {
    const { privateKey, address, message, signedMessage } = ed25519;
    const account = Account.fromPrivateKeyAndAddress({
      privateKey,
      address: AccountAddress.fromString({ input: address }),
    });
    expect(account.sign(message).toString()).toEqual(signedMessage);
    expect(account.verifySignature(message, signedMessage)).toBe(true);
  });
});

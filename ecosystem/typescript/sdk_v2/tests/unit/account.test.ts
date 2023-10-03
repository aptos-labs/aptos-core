// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Account } from "../../src/core/account";
import { AccountAddress } from "../../src/core/account_address";
import { Hex } from "../../src/core/hex";
import { Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature } from "../../src/crypto/ed25519";
import { ed25519 } from "./helper";
import { PublicKey } from "../../src/crypto/asymmetric_crypto";

describe("Account", () => {
  it("should create an instance of Account correctly without error", () => {
    const account = Account.generate();
    expect(account).toBeInstanceOf(Account);
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
    const { mnemonic } = ed25519;
    const address = "0x07968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30";
    const path = "m/44'/637'/0'/0'/0'";
    const newAccount = Account.fromDerivationPath({ path, mnemonic });
    expect(newAccount.accountAddress.toString()).toEqual(address);
  });

  it("should prevent an invalid bip44 path ", () => {
    const { mnemonic } = ed25519;
    const path = "1234";
    expect(() => Account.fromDerivationPath({ path, mnemonic })).toThrow("Invalid derivation path");
  });

  it("should check if a derivation path is valid", () => {
    const validPath = "m/44'/637'/0'/0'/0'"; // Valid path
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

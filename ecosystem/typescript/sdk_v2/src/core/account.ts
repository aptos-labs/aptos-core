// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import nacl from "tweetnacl";
import * as bip39 from "@scure/bip39";
import { AccountAddress } from "./account_address";
import { Memoize, derivePath } from "../utils";
import { Hex } from "./hex";
import { bytesToHex } from "@noble/hashes/utils";
import { Ed25519PublicKey } from "../crypto/ed25519";
import { AuthenticationKey } from "../crypto/authentication_key";
import { HexInput } from "../types";

/**
 * Class for creating and managing account on Aptos network
 *
 * Use this class to create accounts, sign transactions, and more.
 */
export class Account {
  /**
   * signing key of the account, which holds the public and private key
   */
  private readonly _signingKey: nacl.SignKeyPair;

  /**
   * Account address associated with the account
   */
  private readonly _accountAddress: AccountAddress;

  /**
   * Public key of the account
   *
   * @returns Hex - public key of the account
   */
  get publicKey(): Hex {
    return new Hex({ data: this._signingKey.publicKey });
  }

  /**
   * Private key of the account
   *
   * @returns Hex - private key of the account
   */
  get privateKey(): Hex {
    return new Hex({ data: this._signingKey.secretKey });
  }

  /**
   * Address of the account
   *
   * @returns AccountAddress - address of the account
   */
  get accountAddress(): AccountAddress {
    return this._accountAddress;
  }

  /**
   * private constructor for Account
   *
   * This method is private because it should only be called by the factory static methods.
   * @returns Account
   */
  private constructor(keyPair: nacl.SignKeyPair, address: AccountAddress) {
    this._signingKey = keyPair;
    this._accountAddress = address;
  }

  /**
   * Creates new account with random private key and address
   *
   * @returns Account
   */
  static create(): Account {
    const keyPair = nacl.sign.keyPair();
    const address = new AccountAddress({ data: Account.authKey(keyPair.publicKey).toUint8Array() });
    return new Account(keyPair, address);
  }

  /**
   * Creates new account with provided private key
   *
   * @param privateKey Hex - private key of the account
   * @returns Account
   */
  static fromPrivateKey(privateKey: HexInput): Account {
    const privatekeyHex = Hex.fromHexInput({ hexInput: privateKey });
    const keyPair = nacl.sign.keyPair.fromSeed(privatekeyHex.toUint8Array().slice(0, 32));
    const address = new AccountAddress({ data: Account.authKey(keyPair.publicKey).toUint8Array() });
    return new Account(keyPair, address);
  }

  /**
   * Creates new account with provided private key and address
   * This is intended to be used for account that has it's key rotated
   *
   * @param privateKey Hex - private key of the account
   * @param address AccountAddress - address of the account
   * @returns Account
   */
  static fromPrivateKeyAndAddress(privateKey: HexInput, address: AccountAddress): Account {
    const privatekeyHex = Hex.fromHexInput({ hexInput: privateKey });
    const signingKey = nacl.sign.keyPair.fromSeed(privatekeyHex.toUint8Array().slice(0, 32));
    return new Account(signingKey, address);
  }

  /**
   * Creates new account with bip44 path and mnemonics,
   * @param path. (e.g. m/44'/637'/0'/0'/0')
   * Detailed description: {@link https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki}
   * @param mnemonics.
   * @returns AptosAccount
   */
  static fromDerivationPath(path: string, mnemonics: string): Account {
    if (!Account.isValidPath(path)) {
      throw new Error("Invalid derivation path");
    }

    const normalizeMnemonics = mnemonics
      .trim()
      .split(/\s+/)
      .map((part) => part.toLowerCase())
      .join(" ");

    const { key } = derivePath(path, bytesToHex(bip39.mnemonicToSeedSync(normalizeMnemonics)));

    const signingKey = nacl.sign.keyPair.fromSeed(key.slice(0, 32));
    const address = new AccountAddress({ data: Account.authKey(signingKey.publicKey).toUint8Array() });

    return new Account(signingKey, address);
  }

  /**
   * Check's if the derive path is valid
   */
  static isValidPath(path: string): boolean {
    return /^m\/44'\/637'\/[0-9]+'\/[0-9]+'\/[0-9]+'+$/.test(path);
  }

  /**
   * This key enables account owners to rotate their private key(s)
   * associated with the account without changing the address that hosts their account.
   * See here for more info: {@link https://aptos.dev/concepts/accounts#single-signer-authentication}
   * @returns Authentication key for the associated account
   */
  @Memoize()
  static authKey(publicKey: HexInput): Hex {
    const pubKey = new Ed25519PublicKey(publicKey);
    const authKey = AuthenticationKey.fromEd25519PublicKey(pubKey);
    return authKey.data;
  }

  sign(data: HexInput): Hex {
    const hex = Hex.fromHexInput({ hexInput: data });
    const signature = nacl.sign.detached(hex.toUint8Array(), this._signingKey.secretKey);
    return new Hex({ data: signature });
  }

  verifySignature(message: HexInput, signature: HexInput): boolean {
    const rawMessage = Hex.fromHexInput({ hexInput: message }).toUint8Array();
    const rawSignature = Hex.fromHexInput({ hexInput: signature }).toUint8Array();
    return nacl.sign.detached.verify(rawMessage, rawSignature, this._signingKey.publicKey);
  }
}

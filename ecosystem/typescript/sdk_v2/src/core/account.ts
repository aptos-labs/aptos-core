// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import nacl from "tweetnacl";
import * as bip39 from "@scure/bip39";
import { AccountAddress } from "./account_address";
import { Hex } from "./hex";
import { bytesToHex } from "@noble/hashes/utils";
import { HexInput } from "../types";
import { PrivateKey, PublicKey, Signature } from "../crypto/asymmetric_crypto";
import { derivePath } from "../utils/hd-key";
import { AuthenticationKey } from "../crypto/authentication_key";
import { Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature } from "../crypto/ed25519";

/**
 * Class for creating and managing account on Aptos network
 *
 * Use this class to create accounts, sign transactions, and more.
 * Note: Creating an account instance does not create the account onchain.
 */
export class Account {
  /**
   * A private key and public key, associated with the given account
   */
  readonly publicKey: PublicKey;
  readonly privateKey: PrivateKey;

  /**
   * Account address associated with the account
   */
  readonly accountAddress: AccountAddress;

  /**
   * constructor for Account
   *
   * TODO: This constructor uses the nacl library directly, which only works with ed25519 keys.
   * Need to update this to use the new crypto library if new schemes are added.
   *
   * @param args.privateKey PrivateKey - private key of the account
   * @param args.address AccountAddress - address of the account
   *
   * This method is private because it should only be called by the factory static methods.
   * @returns Account
   */
  private constructor(args: { privateKey: PrivateKey; address: AccountAddress }) {
    const { privateKey, address } = args;

    // Derive the public key from the private key
    const keyPair = nacl.sign.keyPair.fromSeed(privateKey.toUint8Array().slice(0, 32));
    this.publicKey = new Ed25519PublicKey({ hexInput: keyPair.publicKey });

    this.privateKey = privateKey;
    this.accountAddress = address;
  }

  /**
   * Derives an account with random private key and address
   *
   * @returns Account
   */
  static generate(): Account {
    const keyPair = nacl.sign.keyPair();
    const privateKey = new Ed25519PrivateKey({ value: keyPair.secretKey.slice(0, 32) });
    const address = new AccountAddress({ data: Account.authKey({ publicKey: keyPair.publicKey }).toUint8Array() });
    return new Account({ privateKey, address });
  }

  /**
   * Derives an account with provided private key
   *
   * @param args.privateKey Hex - private key of the account
   * @returns Account
   */
  static fromPrivateKey(args: { privateKey: HexInput }): Account {
    const privatekeyHex = Hex.fromHexInput({ hexInput: args.privateKey });
    const keyPair = nacl.sign.keyPair.fromSeed(privatekeyHex.toUint8Array().slice(0, 32));
    const privateKey = new Ed25519PrivateKey({ value: keyPair.secretKey.slice(0, 32) });
    const address = new AccountAddress({ data: Account.authKey({ publicKey: keyPair.publicKey }).toUint8Array() });
    return new Account({ privateKey, address });
  }

  /**
   * Derives an account with provided private key and address
   * This is intended to be used for account that has it's key rotated
   *
   * @param args.privateKey Hex - private key of the account
   * @param args.address AccountAddress - address of the account
   * @returns Account
   */
  static fromPrivateKeyAndAddress(args: { privateKey: HexInput; address: AccountAddress }): Account {
    const privatekeyHex = Hex.fromHexInput({ hexInput: args.privateKey });
    const signingKey = nacl.sign.keyPair.fromSeed(privatekeyHex.toUint8Array().slice(0, 32));
    const privateKey = new Ed25519PrivateKey({ value: signingKey.secretKey.slice(0, 32) });
    return new Account({ privateKey, address: args.address });
  }

  /**
   * Derives an account with bip44 path and mnemonics,
   *
   * @param path. (e.g. m/44'/637'/0'/0'/0')
   * Detailed description: {@link https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki}
   * @param mnemonics.
   * @returns AptosAccount
   */
  static fromDerivationPath(args: { path: string; mnemonic: string }): Account {
    const { path, mnemonic } = args;
    if (!Account.isValidPath({ path })) {
      throw new Error("Invalid derivation path");
    }

    const normalizeMnemonics = mnemonic
      .trim()
      .split(/\s+/)
      .map((part) => part.toLowerCase())
      .join(" ");

    const { key } = derivePath(path, bytesToHex(bip39.mnemonicToSeedSync(normalizeMnemonics)));

    const signingKey = nacl.sign.keyPair.fromSeed(key.slice(0, 32));
    const privateKey = new Ed25519PrivateKey({ value: signingKey.secretKey.slice(0, 32) });
    const address = new AccountAddress({ data: Account.authKey({ publicKey: signingKey.publicKey }).toUint8Array() });

    return new Account({ privateKey, address });
  }

  /**
   * Check's if the derive path is valid
   */
  static isValidPath(args: { path: string }): boolean {
    return /^m\/44'\/637'\/[0-9]+'\/[0-9]+'\/[0-9]+'+$/.test(args.path);
  }

  /**
   * This key enables account owners to rotate their private key(s)
   * associated with the account without changing the address that hosts their account.
   * See here for more info: {@link https://aptos.dev/concepts/accounts#single-signer-authentication}
   * @returns Authentication key for the associated account
   */
  static authKey(args: { publicKey: HexInput }): Hex {
    const publicKey = new Ed25519PublicKey({ hexInput: args.publicKey });
    const authKey = AuthenticationKey.fromPublicKey({ publicKey });
    return authKey.data;
  }

  /**
   * Sign the given message with the private key.
   *
   * @param args.data in HexInput format
   * @returns Signature
   */
  sign(args: { data: HexInput }): Signature {
    const signature = this.privateKey.sign({ message: args.data });
    return signature;
  }

  /**
   * Verify the given message and signature with the public key.
   *
   * @param args.message raw message data in HexInput format
   * @param args.signature signed message Signature
   * @returns
   */
  verifySignature(args: { message: HexInput; signature: Signature }): boolean {
    const { message, signature } = args;
    const rawMessage = Hex.fromHexInput({ hexInput: message }).toUint8Array();
    return this.publicKey.verifySignature({ data: rawMessage, signature });
  }
}

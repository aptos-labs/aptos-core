// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import * as bip39 from "@scure/bip39";
import { bytesToHex } from "@noble/hashes/utils";
import { AccountAddress } from "./account_address";
import { Hex } from "./hex";
import { HexInput, SigningScheme } from "../types";
import { PrivateKey, PublicKey, Signature } from "../crypto/asymmetric_crypto";
import { derivePath } from "../utils/hd-key";
import { AuthenticationKey } from "../crypto/authentication_key";
import { Ed25519PrivateKey, Ed25519PublicKey } from "../crypto/ed25519";
import { Secp256k1PrivateKey, Secp256k1PublicKey } from "../crypto/secp256k1";
import { MultiEd25519PublicKey } from "../crypto/multi_ed25519";

/**
 * Class for creating and managing account on Aptos network
 *
 * Use this class to create accounts, sign transactions, and more.
 * Note: Creating an account instance does not create the account onchain.
 */
export class Account {
  /**
   * Public key associated with the account
   */
  readonly publicKey: PublicKey;

  /**
   * Private key associated with the account
   */
  readonly privateKey: PrivateKey;

  /**
   * Account address associated with the account
   */
  readonly accountAddress: AccountAddress;

  /**
   * Signing scheme used to sign transactions
   */
  readonly signingScheme: SigningScheme;

  /**
   * constructor for Account
   *
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
    this.publicKey = privateKey.publicKey();

    // Derive the signing scheme from the public key
    if (this.publicKey instanceof Ed25519PublicKey) {
      this.signingScheme = SigningScheme.Ed25519;
    } else if (this.publicKey instanceof MultiEd25519PublicKey) {
      this.signingScheme = SigningScheme.MultiEd25519;
    } else if (this.publicKey instanceof Secp256k1PublicKey) {
      // Secp256k1
      this.signingScheme = SigningScheme.Secp256k1Ecdsa;
    } else {
      throw new Error("Can not create new Account, unsupported public key type");
    }

    this.privateKey = privateKey;
    this.accountAddress = address;
  }

  /**
   * Derives an account with random private key and address
   *
   * @param args.scheme SigningScheme - type of SigningScheme to use.
   * Currently only Ed25519, MultiEd25519, and Secp256k1 are supported
   *
   * @returns Account
   */
  static generate(args: { scheme: SigningScheme }): Account {
    const { scheme } = args;
    let privateKey: PrivateKey;

    if (scheme === SigningScheme.Ed25519) {
      privateKey = Ed25519PrivateKey.generate();
    } else if (scheme === SigningScheme.Secp256k1Ecdsa) {
      privateKey = Secp256k1PrivateKey.generate();
    } else {
      // TODO: Add support for MultiEd25519
      throw new Error(`Can not generate new Private Key, unsupported signing scheme ${scheme}`);
    }

    const address = new AccountAddress({ data: Account.authKey({ publicKey: privateKey.publicKey() }).toUint8Array() });
    return new Account({ privateKey, address });
  }

  /**
   * Derives an account with provided private key
   *
   * @param args.privateKey Hex - private key of the account
   * @returns Account
   */
  static fromPrivateKey(args: { privateKey: PrivateKey }): Account {
    const { privateKey } = args;
    const publicKey = privateKey.publicKey();
    const authKey = Account.authKey({ publicKey });
    const address = new AccountAddress({ data: authKey.toUint8Array() });
    return Account.fromPrivateKeyAndAddress({ privateKey, address });
  }

  /**
   * Derives an account with provided private key and address
   * This is intended to be used for account that has it's key rotated
   *
   * @param args.privateKey Hex - private key of the account
   * @param args.address AccountAddress - address of the account
   * @returns Account
   */
  static fromPrivateKeyAndAddress(args: { privateKey: PrivateKey; address: AccountAddress }): Account {
    return new Account(args);
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
    const privateKey = new Ed25519PrivateKey({ hexInput: key });
    return Account.fromPrivateKey({ privateKey });
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
  static authKey(args: { publicKey: PublicKey }): Hex {
    const { publicKey } = args;
    const authKey = AuthenticationKey.fromPublicKey({ publicKey });
    return authKey.data;
  }

  /**
   * Sign the given message with the private key.
   *
   * TODO: Add sign transaction or specific types
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
    return this.publicKey.verifySignature({ message: rawMessage, signature });
  }
}

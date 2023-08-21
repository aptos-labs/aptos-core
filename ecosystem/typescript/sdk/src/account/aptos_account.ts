// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import nacl from "tweetnacl";
import * as bip39 from "@scure/bip39";
import { bytesToHex } from "@noble/hashes/utils";
import { sha256 } from "@noble/hashes/sha256";
import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { derivePath } from "../utils/hd-key";
import { HexString, MaybeHexString, Memoize } from "../utils";
import * as Gen from "../generated/index";
import { AccountAddress, AuthenticationKey, Ed25519PublicKey } from "../aptos_types";
import { bcsToBytes } from "../bcs";

export interface AptosAccountObject {
  address?: Gen.HexEncodedBytes;
  publicKeyHex?: Gen.HexEncodedBytes;
  privateKeyHex: Gen.HexEncodedBytes;
}

/**
 * Class for creating and managing Aptos account
 */
export class AptosAccount {
  /**
   * A private key and public key, associated with the given account
   */
  readonly signingKey: nacl.SignKeyPair;

  /**
   * Address associated with the given account
   */
  private readonly accountAddress: HexString;

  static fromAptosAccountObject(obj: AptosAccountObject): AptosAccount {
    return new AptosAccount(HexString.ensure(obj.privateKeyHex).toUint8Array(), obj.address);
  }

  /**
   * Check's if the derive path is valid
   */
  static isValidPath(path: string): boolean {
    return /^m\/44'\/637'\/[0-9]+'\/[0-9]+'\/[0-9]+'+$/.test(path);
  }

  /**
   * Creates new account with bip44 path and mnemonics,
   * @param path. (e.g. m/44'/637'/0'/0'/0')
   * Detailed description: {@link https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki}
   * @param mnemonics.
   * @returns AptosAccount
   */
  static fromDerivePath(path: string, mnemonics: string): AptosAccount {
    if (!AptosAccount.isValidPath(path)) {
      throw new Error("Invalid derivation path");
    }

    const normalizeMnemonics = mnemonics
      .trim()
      .split(/\s+/)
      .map((part) => part.toLowerCase())
      .join(" ");

    const { key } = derivePath(path, bytesToHex(bip39.mnemonicToSeedSync(normalizeMnemonics)));

    return new AptosAccount(key);
  }

  /**
   * Creates new account instance. Constructor allows passing in an address,
   * to handle account key rotation, where auth_key != public_key
   * @param privateKeyBytes  Private key from which account key pair will be generated.
   * If not specified, new key pair is going to be created.
   * @param address Account address (e.g. 0xe8012714cd17606cee7188a2a365eef3fe760be598750678c8c5954eb548a591).
   * If not specified, a new one will be generated from public key
   */
  constructor(privateKeyBytes?: Uint8Array | undefined, address?: MaybeHexString) {
    if (privateKeyBytes) {
      this.signingKey = nacl.sign.keyPair.fromSeed(privateKeyBytes.slice(0, 32));
    } else {
      this.signingKey = nacl.sign.keyPair();
    }
    this.accountAddress = HexString.ensure(address || this.authKey().hex());
  }

  /**
   * This is the key by which Aptos account is referenced.
   * It is the 32-byte of the SHA-3 256 cryptographic hash
   * of the public key(s) concatenated with a signature scheme identifier byte
   * @returns Address associated with the given account
   */
  address(): HexString {
    return this.accountAddress;
  }

  /**
   * This key enables account owners to rotate their private key(s)
   * associated with the account without changing the address that hosts their account.
   * See here for more info: {@link https://aptos.dev/concepts/accounts#single-signer-authentication}
   * @returns Authentication key for the associated account
   */
  @Memoize()
  authKey(): HexString {
    const pubKey = new Ed25519PublicKey(this.signingKey.publicKey);
    const authKey = AuthenticationKey.fromEd25519PublicKey(pubKey);
    return authKey.derivedAddress();
  }

  /**
   * Takes source address and seeds and returns the resource account address
   * @param sourceAddress Address used to derive the resource account
   * @param seed The seed bytes
   * @returns The resource account address
   */
  static getResourceAccountAddress(sourceAddress: MaybeHexString, seed: Uint8Array): HexString {
    const source = bcsToBytes(AccountAddress.fromHex(sourceAddress));

    const bytes = new Uint8Array([...source, ...seed, AuthenticationKey.DERIVE_RESOURCE_ACCOUNT_SCHEME]);

    const hash = sha3Hash.create();
    hash.update(bytes);

    return HexString.fromUint8Array(hash.digest());
  }

  /**
   * Takes creator address and collection name and returns the collection id hash.
   * Collection id hash are generated as sha256 hash of (`creator_address::collection_name`)
   *
   * @param creatorAddress Collection creator address
   * @param collectionName The collection name
   * @returns The collection id hash
   */
  static getCollectionID(creatorAddress: MaybeHexString, collectionName: string): HexString {
    const seed = new TextEncoder().encode(`${creatorAddress}::${collectionName}`);
    const hash = sha256.create();
    hash.update(seed);
    return HexString.fromUint8Array(hash.digest());
  }

  /**
   * This key is generated with Ed25519 scheme.
   * Public key is used to check a signature of transaction, signed by given account
   * @returns The public key for the associated account
   */
  pubKey(): HexString {
    return HexString.fromUint8Array(this.signingKey.publicKey);
  }

  /**
   * Signs specified `buffer` with account's private key
   * @param buffer A buffer to sign
   * @returns A signature HexString
   */
  signBuffer(buffer: Uint8Array): HexString {
    const signature = nacl.sign.detached(buffer, this.signingKey.secretKey);
    return HexString.fromUint8Array(signature);
  }

  /**
   * Signs specified `hexString` with account's private key
   * @param hexString A regular string or HexString to sign
   * @returns A signature HexString
   */
  signHexString(hexString: MaybeHexString): HexString {
    const toSign = HexString.ensure(hexString).toUint8Array();
    return this.signBuffer(toSign);
  }

  /**
   * Verifies the signature of the message with the public key of the account
   * @param message a signed message
   * @param signature the signature of the message
   */
  verifySignature(message: MaybeHexString, signature: MaybeHexString): boolean {
    const rawMessage = HexString.ensure(message).toUint8Array();
    const rawSignature = HexString.ensure(signature).toUint8Array();
    return nacl.sign.detached.verify(rawMessage, rawSignature, this.signingKey.publicKey);
  }

  /**
   * Derives account address, public key and private key
   * @returns AptosAccountObject instance.
   * @example An example of the returned AptosAccountObject object
   * ```
   * {
   *    address: "0xe8012714cd17606cee7188a2a365eef3fe760be598750678c8c5954eb548a591",
   *    publicKeyHex: "0xf56d8524faf79fbc0f48c13aeed3b0ce5dd376b4db93b8130a107c0a5e04ba04",
   *    privateKeyHex: `0x009c9f7c992a06cfafe916f125d8adb7a395fca243e264a8e56a4b3e6accf940
   *      d2b11e9ece3049ce60e3c7b4a1c58aebfa9298e29a30a58a67f1998646135204`
   * }
   * ```
   */
  toPrivateKeyObject(): AptosAccountObject {
    return {
      address: this.address().hex(),
      publicKeyHex: this.pubKey().hex(),
      privateKeyHex: HexString.fromUint8Array(this.signingKey.secretKey.slice(0, 32)).hex(),
    };
  }
}

// Returns an account address as a HexString given either an AptosAccount or a MaybeHexString.
export function getAddressFromAccountOrAddress(accountOrAddress: AptosAccount | MaybeHexString): HexString {
  return accountOrAddress instanceof AptosAccount ? accountOrAddress.address() : HexString.ensure(accountOrAddress);
}

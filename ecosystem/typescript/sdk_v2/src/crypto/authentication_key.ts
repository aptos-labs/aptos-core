// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { AccountAddress, Hex } from "../core";
import { HexInput } from "../types";
import { MultiEd25519PublicKey } from "./multi_ed25519";
import { PublicKey } from "./asymmetric_crypto";
import { Ed25519PublicKey } from "./ed25519";

/**
 * A list of Authentication Key schemes that are supported by Aptos.
 *
 * Keys that start with `Derive` are solely used for deriving account addresses from
 * other data. They are not used for signing transactions.
 */
export enum AuthenticationKeyScheme {
  /**
   * For Ed25519PublicKey
   */
  Ed25519 = 0,
  /**
   * For MultiEd25519PublicKey
   */
  MultiEd25519 = 1,
  /**
   * Derives an address using an AUID, used for objects
   */
  DeriveAuid = 251,
  /**
   * Derives an address from another object address
   */
  DeriveObjectAddressFromObject = 252,
  /**
   * Derives an address from a GUID, used for objects
   */
  DeriveObjectAddressFromGuid = 253,
  /**
   * Derives an address from seed bytes, used for named objects
   */
  DeriveObjectAddressFromSeed = 254,
  /**
   * Derives an address from seed bytes, used for resource accounts
   */
  DeriveResourceAccountAddress = 255,
}

/**
 * Each account stores an authentication key. Authentication key enables account owners to rotate
 * their private key(s) associated with the account without changing the address that hosts their account.
 * @see {@link https://aptos.dev/concepts/accounts | Account Basics}
 *
 * Note: AuthenticationKey only supports Ed25519 and MultiEd25519 public keys for now.
 *
 * Account addresses can be derived from AuthenticationKey
 */
export class AuthenticationKey {
  /**
   * An authentication key is always a SHA3-256 hash of data, and is always 32 bytes.
   */
  static readonly LENGTH: number = 32;

  /**
   * The raw bytes of the authentication key.
   */
  public readonly data: Hex;

  constructor(args: { data: HexInput }) {
    const { data } = args;
    const hex = Hex.fromHexInput({ hexInput: data });
    if (hex.toUint8Array().length !== AuthenticationKey.LENGTH) {
      throw new Error(`Authentication Key length should be ${AuthenticationKey.LENGTH}`);
    }
    this.data = hex;
  }

  toString(): string {
    return this.data.toString();
  }

  toUint8Array(): Uint8Array {
    return this.data.toUint8Array();
  }

  /**
   * Creates an AuthenticationKey from seed bytes and a scheme
   *
   * This allows for the creation of AuthenticationKeys that are not derived from Public Keys directly
   * @param args
   */
  private static fromBytesAndScheme(args: { bytes: HexInput; scheme: AuthenticationKeyScheme }) {
    const { bytes, scheme } = args;
    const inputBytes = Hex.fromHexInput({ hexInput: bytes }).toUint8Array();
    const authKeyBytes = new Uint8Array(inputBytes.length + 1);
    authKeyBytes.set(inputBytes);
    authKeyBytes.set([scheme], inputBytes.length);

    const hash = sha3Hash.create();
    hash.update(authKeyBytes);

    return new AuthenticationKey({ data: hash.digest() });
  }

  /**
   * Converts a PublicKey(s) to AuthenticationKey
   *
   * @param publicKey
   * @returns AuthenticationKey
   */
  static fromPublicKey(args: { publicKey: PublicKey }): AuthenticationKey {
    const { publicKey } = args;

    let scheme: number;
    if (publicKey instanceof Ed25519PublicKey) {
      scheme = AuthenticationKeyScheme.Ed25519.valueOf();
    } else if (publicKey instanceof MultiEd25519PublicKey) {
      scheme = AuthenticationKeyScheme.MultiEd25519.valueOf();
    } else {
      throw new Error("Unsupported authentication key scheme");
    }

    const pubKeyBytes = publicKey.toUint8Array();
    return AuthenticationKey.fromBytesAndScheme({ bytes: pubKeyBytes, scheme });
  }

  /**
   * Derives an account address from AuthenticationKey. Since current AccountAddress is 32 bytes,
   * AuthenticationKey bytes are directly translated to AccountAddress.
   *
   * @returns AccountAddress
   */
  derivedAddress(): AccountAddress {
    return new AccountAddress({ data: this.data.toUint8Array() });
  }
}

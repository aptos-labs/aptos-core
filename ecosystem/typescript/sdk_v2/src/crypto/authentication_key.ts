// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { AccountAddress, Hex } from "../core";
import { HexInput } from "../types";
import { MultiEd25519PublicKey } from "./multi_ed25519";
import { PublicKey } from "./asymmetric_crypto";
import { Ed25519PublicKey } from "./ed25519";

export enum AuthenticationKeyScheme {
  Ed25519 = 0,
  MultiEd25519 = 1,
  DeriveAuid = 251,
  DeriveObjectAddressFromObject = 252,
  DeriveObjectAddressFromGuid = 253,
  DeriveObjectAddressFromSeed = 254,
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
  // Length of AuthenticationKey in bytes(Uint8Array)
  static readonly LENGTH: number = 32;

  // Actual data of AuthenticationKey, in Hex format
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

  static fromBytesAndScheme(args: { hexInput: HexInput; scheme: AuthenticationKeyScheme }) {
    const { hexInput, scheme } = args;
    const inputBytes = Hex.fromHexInput({ hexInput }).toUint8Array();
    const bytes = new Uint8Array(inputBytes.length + 1);
    bytes.set(inputBytes);
    bytes.set([scheme], inputBytes.length);

    const hash = sha3Hash.create();
    hash.update(bytes);

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
    return AuthenticationKey.fromBytesAndScheme({ hexInput: pubKeyBytes, scheme });
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

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { AccountAddress, Hex } from "../core";
import { HexInput } from "../types";
import { MultiEd25519PublicKey } from "./multi_ed25519";
import { Ed25519PublicKey } from "./ed25519";

/**
 * Each account stores an authentication key. Authentication key enables account owners to rotate
 * their private key(s) associated with the account without changing the address that hosts their account.
 * @see {@link * https://aptos.dev/concepts/accounts | Account Basics}
 *
 * Account addresses can be derived from AuthenticationKey
 */
export class AuthenticationKey {
  static readonly LENGTH: number = 32;

  static readonly MULTI_ED25519_SCHEME: number = 1;

  static readonly ED25519_SCHEME: number = 0;

  static readonly DERIVE_RESOURCE_ACCOUNT_SCHEME: number = 255;

  readonly data: Hex;

  constructor(hexInput: HexInput) {
    const hex = Hex.fromHexInput({ hexInput });
    if (hex.toUint8Array().length !== AuthenticationKey.LENGTH) {
      throw new Error("Expected a hexinput of length 32");
    }
    this.data = hex;
  }

  /**
   * Converts a K-of-N MultiEd25519PublicKey to AuthenticationKey with:
   * `auth_key = sha3-256(p_1 | … | p_n | K | 0x01)`. `K` represents the K-of-N required for
   * authenticating the transaction. `0x01` is the 1-byte scheme for multisig.
   */
  static fromMultiEd25519PublicKey(publicKey: MultiEd25519PublicKey): AuthenticationKey {
    const pubKeyBytes = publicKey.toUint8Array();

    const bytes = new Uint8Array(pubKeyBytes.length + 1);
    bytes.set(pubKeyBytes);
    bytes.set([AuthenticationKey.MULTI_ED25519_SCHEME], pubKeyBytes.length);

    const hash = sha3Hash.create();
    hash.update(bytes);

    return new AuthenticationKey(hash.digest());
  }

  static fromEd25519PublicKey(publicKey: Ed25519PublicKey): AuthenticationKey {
    const pubKeyBytes = publicKey.value.toUint8Array();

    const bytes = new Uint8Array(pubKeyBytes.length + 1);
    bytes.set(pubKeyBytes);
    bytes.set([AuthenticationKey.ED25519_SCHEME], pubKeyBytes.length);

    const hash = sha3Hash.create();
    hash.update(bytes);

    return new AuthenticationKey(hash.digest());
  }

  /**
   * Derives an account address from AuthenticationKey. Since current AccountAddress is 32 bytes,
   * AuthenticationKey bytes are directly translated to AccountAddress.
   */
  derivedAddress(): AccountAddress {
    return new AccountAddress({ data: this.data.toUint8Array() });
  }
}
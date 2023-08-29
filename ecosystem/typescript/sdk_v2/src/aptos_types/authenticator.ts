// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable @typescript-eslint/naming-convention */
import { AccountAddress } from "./account_address";
import { Serializer, Deserializer } from "../bcs";
import { Ed25519PublicKey, Ed25519Signature } from "./ed25519";
import { MultiEd25519PublicKey, MultiEd25519Signature } from "./multi_ed25519";

export abstract class TransactionAuthenticator {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TransactionAuthenticator {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TransactionAuthenticatorEd25519.load(deserializer);
      case 1:
        return TransactionAuthenticatorMultiEd25519.load(deserializer);
      case 2:
        return TransactionAuthenticatorMultiAgent.load(deserializer);
      case 3:
        return TransactionAuthenticatorFeePayer.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionAuthenticator: ${index}`);
    }
  }
}

export class TransactionAuthenticatorEd25519 extends TransactionAuthenticator {
  /**
   * An authenticator for single signature.
   *
   * @param public_key Client's public key.
   * @param signature Signature of a raw transaction.
   * @see {@link https://aptos.dev/guides/creating-a-signed-transaction/ | Creating a Signed Transaction}
   * for details about generating a signature.
   */
  constructor(public readonly public_key: Ed25519PublicKey, public readonly signature: Ed25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    serializer.serialize(this.public_key);
    serializer.serialize(this.signature);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorEd25519 {
    const public_key = deserializer.deserialize(Ed25519PublicKey);
    const signature = deserializer.deserialize(Ed25519Signature);
    return new TransactionAuthenticatorEd25519(public_key, signature);
  }
}

export class TransactionAuthenticatorMultiEd25519 extends TransactionAuthenticator {
  /**
   * An authenticator for multiple signatures.
   *
   * @param public_key
   * @param signature
   *
   */
  constructor(public readonly public_key: MultiEd25519PublicKey, public readonly signature: MultiEd25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    serializer.serialize(this.public_key);
    serializer.serialize(this.signature);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorMultiEd25519 {
    const public_key = deserializer.deserialize(MultiEd25519PublicKey);
    const signature = deserializer.deserialize(MultiEd25519Signature);
    return new TransactionAuthenticatorMultiEd25519(public_key, signature);
  }
}

export class TransactionAuthenticatorMultiAgent extends TransactionAuthenticator {
  constructor(
    public readonly sender: AccountAuthenticator,
    public readonly secondary_signer_addresses: Array<AccountAddress>,
    public readonly secondary_signers: Array<AccountAuthenticator>,
  ) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer
        .serializeU32AsUleb128(2)
        .serialize(this.sender)
        .serializeVector<AccountAddress>(this.secondary_signer_addresses)
        .serializeVector<AccountAuthenticator>(this.secondary_signers);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorMultiAgent {
    const sender = deserializer.deserialize(AccountAuthenticator);
    const secondary_signer_addresses = deserializer.deserializeVector(AccountAddress);
    const secondary_signers = deserializer.deserializeVector(AccountAuthenticator);
    return new TransactionAuthenticatorMultiAgent(sender, secondary_signer_addresses, secondary_signers);
  }
}

export class TransactionAuthenticatorFeePayer extends TransactionAuthenticator {
  constructor(
    public readonly sender: AccountAuthenticator,
    public readonly secondary_signer_addresses: Array<AccountAddress>,
    public readonly secondary_signers: Array<AccountAuthenticator>,
    public readonly fee_payer: { address: AccountAddress; authenticator: AccountAuthenticator },
  ) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer
        .serializeU32AsUleb128(3)
        .serialize(this.sender)
        .serializeVector<AccountAddress>(this.secondary_signer_addresses)
        .serializeVector<AccountAuthenticator>(this.secondary_signers)
        .serialize(this.fee_payer.address)
        .serialize(this.fee_payer.authenticator);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorMultiAgent {
    const sender = deserializer.deserialize(AccountAuthenticator);
    const secondary_signer_addresses = deserializer.deserializeVector(AccountAddress);
    const secondary_signers = deserializer.deserializeVector(AccountAuthenticator);
    const address = deserializer.deserialize(AccountAddress);
    const authenticator = deserializer.deserialize(AccountAuthenticator);
    const fee_payer = { address, authenticator };
    return new TransactionAuthenticatorFeePayer(sender, secondary_signer_addresses, secondary_signers, fee_payer);
  }
}

export abstract class AccountAuthenticator {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): AccountAuthenticator {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return AccountAuthenticatorEd25519.load(deserializer);
      case 1:
        return AccountAuthenticatorMultiEd25519.load(deserializer);
      default:
        throw new Error(`Unknown variant index for AccountAuthenticator: ${index}`);
    }
  }
}

export class AccountAuthenticatorEd25519 extends AccountAuthenticator {
  constructor(public readonly public_key: Ed25519PublicKey, public readonly signature: Ed25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    serializer.serialize(this.public_key);
    serializer.serialize(this.signature);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorEd25519 {
    const public_key = deserializer.deserialize(Ed25519PublicKey);
    const signature = deserializer.deserialize(Ed25519Signature);
    return new AccountAuthenticatorEd25519(public_key, signature);
  }
}

export class AccountAuthenticatorMultiEd25519 extends AccountAuthenticator {
  constructor(public readonly public_key: MultiEd25519PublicKey, public readonly signature: MultiEd25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    serializer.serialize(this.public_key);
    serializer.serialize(this.signature);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorMultiEd25519 {
    const public_key = deserializer.deserialize(MultiEd25519PublicKey);
    const signature = deserializer.deserialize(MultiEd25519Signature);
    return new AccountAuthenticatorMultiEd25519(public_key, signature);
  }
}

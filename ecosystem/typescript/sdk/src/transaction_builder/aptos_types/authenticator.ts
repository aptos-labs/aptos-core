/* eslint-disable @typescript-eslint/naming-convention */
/* eslint-disable max-classes-per-file */
import { Serializer, Deserializer, Bytes, Seq, deserializeVector, serializeVector } from '../bcs';
import { AccountAddress } from './account_address';

export abstract class TransactionAuthenticator {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TransactionAuthenticator {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TransactionAuthenticatorVariantEd25519.load(deserializer);
      case 1:
        return TransactionAuthenticatorVariantMultiEd25519.load(deserializer);
      case 2:
        return TransactionAuthenticatorVariantMultiAgent.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionAuthenticator: ${index}`);
    }
  }
}

export class TransactionAuthenticatorVariantEd25519 extends TransactionAuthenticator {
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
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorVariantEd25519 {
    const public_key = Ed25519PublicKey.deserialize(deserializer);
    const signature = Ed25519Signature.deserialize(deserializer);
    return new TransactionAuthenticatorVariantEd25519(public_key, signature);
  }
}

export class TransactionAuthenticatorVariantMultiEd25519 extends TransactionAuthenticator {
  /**
   * An authenticator for multiple signatures.
   *
   * @param public_key BCS bytes for a list of public keys.
   *
   * @example
   * Developers must manually construct the input to get the BCS bytes.
   * See below code example for the BCS input.
   * ```ts
   * interface  MultiEd25519PublicKey {
   *   // A list of public keys
   *   public_keys: Uint8Array[],
   *   // At least `threshold` signatures must be valid
   *   threshold: Uint8,
   * }
   * ```
   * @param signature BCS bytes of multiple signatures.
   *
   * @example
   * Developers must manually construct the input to get the BCS bytes.
   * See below code example for the BCS input.
   * ```ts
   * interface  MultiEd25519Signature {
   *   // A list of signatures
   *   signatures: Uint8Array[],
   *   // 4 bytes, at most 32 signatures are supported.
   *   // If Nth bit value is `1`, the Nth signature should be provided in `signatures`.
   *   // Bits are read from left to right.
   *   bitmap: Uint8Array,
   * }
   * ```
   */
  constructor(public readonly public_key: MultiEd25519PublicKey, public readonly signature: MultiEd25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorVariantMultiEd25519 {
    const public_key = MultiEd25519PublicKey.deserialize(deserializer);
    const signature = MultiEd25519Signature.deserialize(deserializer);
    return new TransactionAuthenticatorVariantMultiEd25519(public_key, signature);
  }
}

export class TransactionAuthenticatorVariantMultiAgent extends TransactionAuthenticator {
  constructor(
    public readonly sender: AccountAuthenticator,
    public readonly secondary_signer_addresses: Seq<AccountAddress>,
    public readonly secondary_signers: Seq<AccountAuthenticator>,
  ) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
    this.sender.serialize(serializer);
    serializeVector<AccountAddress>(this.secondary_signer_addresses, serializer);
    serializeVector<AccountAuthenticator>(this.secondary_signers, serializer);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorVariantMultiAgent {
    const sender = AccountAuthenticator.deserialize(deserializer);
    const secondary_signer_addresses = deserializeVector(deserializer, AccountAddress);
    const secondary_signers = deserializeVector(deserializer, AccountAuthenticator);
    return new TransactionAuthenticatorVariantMultiAgent(sender, secondary_signer_addresses, secondary_signers);
  }
}

export abstract class AccountAuthenticator {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): AccountAuthenticator {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return AccountAuthenticatorVariantEd25519.load(deserializer);
      case 1:
        return AccountAuthenticatorVariantMultiEd25519.load(deserializer);
      default:
        throw new Error(`Unknown variant index for AccountAuthenticator: ${index}`);
    }
  }
}

export class AccountAuthenticatorVariantEd25519 extends AccountAuthenticator {
  constructor(public readonly public_key: Ed25519PublicKey, public readonly signature: Ed25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorVariantEd25519 {
    const public_key = Ed25519PublicKey.deserialize(deserializer);
    const signature = Ed25519Signature.deserialize(deserializer);
    return new AccountAuthenticatorVariantEd25519(public_key, signature);
  }
}

export class AccountAuthenticatorVariantMultiEd25519 extends AccountAuthenticator {
  constructor(public readonly public_key: MultiEd25519PublicKey, public readonly signature: MultiEd25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorVariantMultiEd25519 {
    const public_key = MultiEd25519PublicKey.deserialize(deserializer);
    const signature = MultiEd25519Signature.deserialize(deserializer);
    return new AccountAuthenticatorVariantMultiEd25519(public_key, signature);
  }
}

export class Ed25519PublicKey {
  constructor(public readonly value: Bytes) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.value);
  }

  static deserialize(deserializer: Deserializer): Ed25519PublicKey {
    const value = deserializer.deserializeBytes();
    return new Ed25519PublicKey(value);
  }
}

export class Ed25519Signature {
  constructor(public readonly value: Bytes) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.value);
  }

  static deserialize(deserializer: Deserializer): Ed25519Signature {
    const value = deserializer.deserializeBytes();
    return new Ed25519Signature(value);
  }
}

export class MultiEd25519PublicKey {
  constructor(public readonly value: Bytes) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.value);
  }

  static deserialize(deserializer: Deserializer): MultiEd25519PublicKey {
    const value = deserializer.deserializeBytes();
    return new MultiEd25519PublicKey(value);
  }
}

export class MultiEd25519Signature {
  constructor(public readonly value: Bytes) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.value);
  }

  static deserialize(deserializer: Deserializer): MultiEd25519Signature {
    const value = deserializer.deserializeBytes();
    return new MultiEd25519Signature(value);
  }
}

/* eslint-disable @typescript-eslint/naming-convention */
/* eslint-disable max-classes-per-file */
import assert from 'assert';
import { Serializer, Deserializer, Bytes, Seq, deserializeVector, serializeVector, Uint8 } from '../bcs';
import { AccountAddress } from './account_address';

/**
 * MultiEd25519 currently supports at most 32 signatures.
 */
const MAX_SIGNATURES_SUPPORTED = 32;

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
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorEd25519 {
    const public_key = Ed25519PublicKey.deserialize(deserializer);
    const signature = Ed25519Signature.deserialize(deserializer);
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
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorMultiEd25519 {
    const public_key = MultiEd25519PublicKey.deserialize(deserializer);
    const signature = MultiEd25519Signature.deserialize(deserializer);
    return new TransactionAuthenticatorMultiEd25519(public_key, signature);
  }
}

export class TransactionAuthenticatorMultiAgent extends TransactionAuthenticator {
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

  static load(deserializer: Deserializer): TransactionAuthenticatorMultiAgent {
    const sender = AccountAuthenticator.deserialize(deserializer);
    const secondary_signer_addresses = deserializeVector(deserializer, AccountAddress);
    const secondary_signers = deserializeVector(deserializer, AccountAuthenticator);
    return new TransactionAuthenticatorMultiAgent(sender, secondary_signer_addresses, secondary_signers);
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
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorEd25519 {
    const public_key = Ed25519PublicKey.deserialize(deserializer);
    const signature = Ed25519Signature.deserialize(deserializer);
    return new AccountAuthenticatorEd25519(public_key, signature);
  }
}

export class AccountAuthenticatorMultiEd25519 extends AccountAuthenticator {
  constructor(public readonly public_key: MultiEd25519PublicKey, public readonly signature: MultiEd25519Signature) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorMultiEd25519 {
    const public_key = MultiEd25519PublicKey.deserialize(deserializer);
    const signature = MultiEd25519Signature.deserialize(deserializer);
    return new AccountAuthenticatorMultiEd25519(public_key, signature);
  }
}

export class Ed25519PublicKey {
  static readonly LENGTH: number = 32;

  readonly value: Bytes;

  constructor(value: Bytes) {
    if (value.length !== Ed25519PublicKey.LENGTH) {
      throw new Error(`Ed25519PublicKey length should be ${Ed25519PublicKey.LENGTH}`);
    }
    this.value = value;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.value);
  }

  static deserialize(deserializer: Deserializer): Ed25519PublicKey {
    const value = deserializer.deserializeBytes();
    return new Ed25519PublicKey(value);
  }
}

export class Ed25519Signature {
  static readonly LENGTH = 64;

  constructor(public readonly value: Bytes) {
    if (value.length !== Ed25519Signature.LENGTH) {
      throw new Error(`Ed25519Signature length should be ${Ed25519Signature.LENGTH}`);
    }
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.value);
  }

  static deserialize(deserializer: Deserializer): Ed25519Signature {
    const value = deserializer.deserializeBytes();
    return new Ed25519Signature(value);
  }
}

export class MultiEd25519PublicKey {
  /**
   * Public key for a K-of-N multisig transaction. A K-of-N multisig transaction means that for such a
   * transaction to be executed, at least K out of the N authorized signers have signed the transaction
   * and passed the check conducted by the chain.
   *
   * @see {@link
   * https://aptos.dev/guides/creating-a-signed-transaction#multisignature-transactions | Creating a Signed Transaction}
   *
   * @param public_keys A list of public keys
   * @param threshold At least "threshold" signatures must be valid
   */
  constructor(public readonly public_keys: Seq<Ed25519PublicKey>, public readonly threshold: Uint8) {
    if (threshold > MAX_SIGNATURES_SUPPORTED) {
      throw new Error(`"threshold" cannot be larger than ${MAX_SIGNATURES_SUPPORTED}`);
    }
  }

  toBytes(): Bytes {
    const bytes = new Uint8Array(this.public_keys.length * Ed25519PublicKey.LENGTH + 1);
    this.public_keys.forEach((k: Ed25519PublicKey, i: number) => {
      bytes.set(k.value, i * Ed25519PublicKey.LENGTH);
    });

    bytes[this.public_keys.length * Ed25519PublicKey.LENGTH] = this.threshold;

    return bytes;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toBytes());
  }

  static deserialize(deserializer: Deserializer): MultiEd25519PublicKey {
    const bytes = deserializer.deserializeBytes();
    const threshold = bytes[bytes.length - 1];

    const keys: Seq<Ed25519PublicKey> = [];

    for (let i = 0; i < bytes.length; i += Ed25519PublicKey.LENGTH) {
      const begin = i * Ed25519PublicKey.LENGTH;
      keys.push(new Ed25519PublicKey(bytes.subarray(begin, begin + Ed25519PublicKey.LENGTH)));
    }
    return new MultiEd25519PublicKey(keys, threshold);
  }
}

export class MultiEd25519Signature {
  static BITMAP_LEN: Uint8 = 4;

  /**
   * Signature for a K-of-N multisig transaction.
   *
   * @see {@link
   * https://aptos.dev/guides/creating-a-signed-transaction#multisignature-transactions | Creating a Signed Transaction}
   *
   * @param signatures A list of ed25519 signatures
   * @param bitmap 4 bytes, at most 32 signatures are supported. If Nth bit value is `1`, the Nth
   * signature should be provided in `signatures`. Bits are read from left to right
   */
  constructor(public readonly signatures: Seq<Ed25519Signature>, public readonly bitmap: Uint8Array) {
    assert(bitmap.length === MultiEd25519Signature.BITMAP_LEN);
  }

  toBytes(): Bytes {
    const bytes = new Uint8Array(this.signatures.length * Ed25519Signature.LENGTH + MultiEd25519Signature.BITMAP_LEN);
    this.signatures.forEach((k: Ed25519Signature, i: number) => {
      bytes.set(k.value, i * Ed25519Signature.LENGTH);
    });

    bytes.set(this.bitmap, this.signatures.length * Ed25519Signature.LENGTH);

    return bytes;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.toBytes());
  }

  static deserialize(deserializer: Deserializer): MultiEd25519Signature {
    const bytes = deserializer.deserializeBytes();
    const bitmap = bytes.subarray(bytes.length - 4);

    const sigs: Seq<Ed25519Signature> = [];

    for (let i = 0; i < bytes.length; i += Ed25519Signature.LENGTH) {
      const begin = i * Ed25519Signature.LENGTH;
      sigs.push(new Ed25519Signature(bytes.subarray(begin, begin + Ed25519Signature.LENGTH)));
    }
    return new MultiEd25519Signature(sigs, bitmap);
  }
}

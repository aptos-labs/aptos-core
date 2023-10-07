/* eslint-disable @typescript-eslint/naming-convention */

import { Serializer, Deserializer, Serializable } from "../../bcs";
import { Ed25519PublicKey, Ed25519Signature } from "../../crypto/ed25519";
import { MultiEd25519PublicKey, MultiEd25519Signature } from "../../crypto/multi_ed25519";
import { Secp256k1PublicKey, Secp256k1Signature } from "../../crypto/secp256k1";
import { AccountAuthenticatorVariant } from "../../types";

export abstract class AccountAuthenticator extends Serializable {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): AccountAuthenticator {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case AccountAuthenticatorVariant.Ed25519:
        return AccountAuthenticatorEd25519.load(deserializer);
      case AccountAuthenticatorVariant.MultiEd25519:
        return AccountAuthenticatorMultiEd25519.load(deserializer);
      case AccountAuthenticatorVariant.Secp256k1:
        return AccountAuthenticatorSecp256k1.load(deserializer);
      default:
        throw new Error(`Unknown variant index for AccountAuthenticator: ${index}`);
    }
  }
}

export class AccountAuthenticatorEd25519 extends AccountAuthenticator {
  public readonly public_key: Ed25519PublicKey;

  public readonly signature: Ed25519Signature;

  constructor(public_key: Ed25519PublicKey, signature: Ed25519Signature) {
    super();
    this.public_key = public_key;
    this.signature = signature;
  }

  /**
   * Transaction authenticator Ed25519 for a multi signers transaction
   *
   * @param public_key Account's Ed25519 public key.
   * @param signature Account's Ed25519 signature
   *
   */
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(AccountAuthenticatorVariant.Ed25519);
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
  public readonly public_key: MultiEd25519PublicKey;

  public readonly signature: MultiEd25519Signature;

  /**
   * Transaction authenticator Multi Ed25519 for a multi signers transaction
   *
   * @param public_key Account's MultiEd25519 public key.
   * @param signature Account's MultiEd25519 signature
   *
   */
  constructor(public_key: MultiEd25519PublicKey, signature: MultiEd25519Signature) {
    super();
    this.public_key = public_key;
    this.signature = signature;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(AccountAuthenticatorVariant.MultiEd25519);
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorMultiEd25519 {
    const public_key = MultiEd25519PublicKey.deserialize(deserializer);
    const signature = MultiEd25519Signature.deserialize(deserializer);
    return new AccountAuthenticatorMultiEd25519(public_key, signature);
  }
}

export class AccountAuthenticatorSecp256k1 extends AccountAuthenticator {
  public readonly public_key: Secp256k1PublicKey;

  public readonly signature: Secp256k1Signature;

  constructor(public_key: Secp256k1PublicKey, signature: Secp256k1Signature) {
    super();
    this.public_key = public_key;
    this.signature = signature;
  }

  /**
   * A Secp256k1 AccountAuthenticator for a single signer
   *
   * @param public_key A Secp256k1 public key
   * @param signature A Secp256k1 signature
   *
   */
  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(AccountAuthenticatorVariant.Secp256k1);
    this.public_key.serialize(serializer);
    this.signature.serialize(serializer);
  }

  static load(deserializer: Deserializer): AccountAuthenticatorSecp256k1 {
    const public_key = Secp256k1PublicKey.deserialize(deserializer);
    const signature = Secp256k1Signature.deserialize(deserializer);
    return new AccountAuthenticatorSecp256k1(public_key, signature);
  }
}

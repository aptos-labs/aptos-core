/* eslint-disable @typescript-eslint/naming-convention */

import { Deserializer, Serializer } from "../../bcs";
import { AccountAddress } from "../../core";
import { Ed25519PublicKey, Ed25519Signature } from "../../crypto/ed25519";
import { MultiEd25519PublicKey, MultiEd25519Signature } from "../../crypto/multi_ed25519";
import { TransactionAuthenticatorVariant } from "../../types";
import { AccountAuthenticator } from "./account";

export abstract class TransactionAuthenticator {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TransactionAuthenticator {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case TransactionAuthenticatorVariant.TransactionAuthenticatorEd25519:
        return TransactionAuthenticatorEd25519.load(deserializer);
      case TransactionAuthenticatorVariant.TransactionAuthenticatorMultiEd25519:
        return TransactionAuthenticatorMultiEd25519.load(deserializer);
      case TransactionAuthenticatorVariant.TransactionAuthenticatorMultiAgent:
        return TransactionAuthenticatorMultiAgent.load(deserializer);
      case TransactionAuthenticatorVariant.TransactionAuthenticatorFeePayer:
        return TransactionAuthenticatorFeePayer.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionAuthenticator: ${index}`);
    }
  }
}

export class TransactionAuthenticatorEd25519 extends TransactionAuthenticator {
  public readonly public_key: Ed25519PublicKey;

  public readonly signature: Ed25519Signature;

  /**
   * Transaction authenticator Ed25519 for a single signer transaction
   *
   * @param public_key Client's public key.
   * @param signature Ed25519 signature of a raw transaction.
   * @see {@link https://aptos.dev/integration/creating-a-signed-transaction | Creating a Signed Transaction}
   * for details about generating a signature.
   */
  constructor(public_key: Ed25519PublicKey, signature: Ed25519Signature) {
    super();
    this.public_key = public_key;
    this.signature = signature;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(TransactionAuthenticatorVariant.TransactionAuthenticatorEd25519);
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
  public readonly public_key: MultiEd25519PublicKey;

  public readonly signature: MultiEd25519Signature;

  /**
   * Transaction authenticator Ed25519 for a multi signers transaction
   *
   * @param public_key Client's public key.
   * @param signature Multi Ed25519 signature of a raw transaction.
   *
   */
  constructor(public_key: MultiEd25519PublicKey, signature: MultiEd25519Signature) {
    super();
    this.public_key = public_key;
    this.signature = signature;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(TransactionAuthenticatorVariant.TransactionAuthenticatorMultiEd25519);
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
  public readonly sender: AccountAuthenticator;

  public readonly secondary_signer_addresses: Array<AccountAddress>;

  public readonly secondary_signers: Array<AccountAuthenticator>;

  /**
   * Transaction authenticator for a multi agent transaction
   *
   * @param sender Sender account authenticator
   * @param secondary_signer_addresses Secondary signers address
   * @param secondary_signers Secondary signers account authenticators
   *
   */
  constructor(
    sender: AccountAuthenticator,
    secondary_signer_addresses: Array<AccountAddress>,
    secondary_signers: Array<AccountAuthenticator>,
  ) {
    super();
    this.sender = sender;
    this.secondary_signer_addresses = secondary_signer_addresses;
    this.secondary_signers = secondary_signers;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(TransactionAuthenticatorVariant.TransactionAuthenticatorMultiAgent);
    this.sender.serialize(serializer);
    serializer.serializeVector<AccountAddress>(this.secondary_signer_addresses);
    serializer.serializeVector<AccountAuthenticator>(this.secondary_signers);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorMultiAgent {
    const sender = AccountAuthenticator.deserialize(deserializer);
    const secondary_signer_addresses = deserializer.deserializeVector(AccountAddress);
    const secondary_signers = deserializer.deserializeVector(AccountAuthenticator);
    return new TransactionAuthenticatorMultiAgent(sender, secondary_signer_addresses, secondary_signers);
  }
}

export class TransactionAuthenticatorFeePayer extends TransactionAuthenticator {
  public readonly sender: AccountAuthenticator;

  public readonly secondary_signer_addresses: Array<AccountAddress>;

  public readonly secondary_signers: Array<AccountAuthenticator>;

  public readonly fee_payer: { address: AccountAddress; authenticator: AccountAuthenticator };

  /**
   * Transaction authenticator for a fee payer transaction
   *
   * @param sender Sender account authenticator
   * @param secondary_signer_addresses Secondary signers address
   * @param secondary_signers Secondary signers account authenticators
   * @param fee_payer Object of the fee payer account address and the fee payer authentication
   *
   */
  constructor(
    sender: AccountAuthenticator,
    secondary_signer_addresses: Array<AccountAddress>,
    secondary_signers: Array<AccountAuthenticator>,
    fee_payer: { address: AccountAddress; authenticator: AccountAuthenticator },
  ) {
    super();
    this.sender = sender;
    this.secondary_signer_addresses = secondary_signer_addresses;
    this.secondary_signers = secondary_signers;
    this.fee_payer = fee_payer;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(TransactionAuthenticatorVariant.TransactionAuthenticatorFeePayer);
    this.sender.serialize(serializer);
    serializer.serializeVector<AccountAddress>(this.secondary_signer_addresses);
    serializer.serializeVector<AccountAuthenticator>(this.secondary_signers);
    this.fee_payer.address.serialize(serializer);
    this.fee_payer.authenticator.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionAuthenticatorMultiAgent {
    const sender = AccountAuthenticator.deserialize(deserializer);
    const secondary_signer_addresses = deserializer.deserializeVector(AccountAddress);
    const secondary_signers = deserializer.deserializeVector(AccountAuthenticator);
    const address = AccountAddress.deserialize(deserializer);
    const authenticator = AccountAuthenticator.deserialize(deserializer);
    const fee_payer = { address, authenticator };
    return new TransactionAuthenticatorFeePayer(sender, secondary_signer_addresses, secondary_signers, fee_payer);
  }
}

import * as SHA3 from "js-sha3";
import {
  Ed25519PublicKey,
  Ed25519Signature,
  RawTransaction,
  SignedTransaction,
  TransactionAuthenticatorVariantEd25519,
} from "./aptos_types";
import { bcsToBytes, bytes } from "./bcs";

const SALT = "APTOS::RawTransaction";

export type SigningMessage = Buffer;
export type TransactionSignature = Uint8Array;

/** Function that takes in a Signing Message (serialized raw transaction)
 *  and returns a signature
 */
export type SigningFn = (txn: SigningMessage) => TransactionSignature;

class TransactionBuilder<F extends SigningFn> {
  private readonly signingFunction: F;

  private readonly publicKey: Uint8Array;

  constructor(signingFunction: F, publicKey: Uint8Array) {
    this.signingFunction = signingFunction;
    this.publicKey = publicKey;
  }

  /** Generates a Signing Message out of a raw transaction. */
  static getSigningMessage(rawTxn: RawTransaction): SigningMessage {
    const hash = SHA3.sha3_256.create();
    hash.update(Buffer.from(SALT));

    const prefix = new Uint8Array(hash.arrayBuffer());

    return Buffer.from([...prefix, ...bcsToBytes(rawTxn)]);
  }

  private signInternal(rawTxn: RawTransaction): SignedTransaction {
    const signingMessage = TransactionBuilder.getSigningMessage(rawTxn);
    const signatureRaw = this.signingFunction(signingMessage);

    const signature = new Ed25519Signature(signatureRaw);

    const authenticator = new TransactionAuthenticatorVariantEd25519(new Ed25519PublicKey(this.publicKey), signature);

    return new SignedTransaction(rawTxn, authenticator);
  }

  /** Signs a raw transaction and returns a bcs serialized transaction. */
  sign(rawTxn: RawTransaction): bytes {
    return bcsToBytes(this.signInternal(rawTxn));
  }
}

/** Just a syntatic sugar.
 *  TODO: add default signing function for Ed25519 */
export class TransactionBuilderEd25519 extends TransactionBuilder<SigningFn> {}

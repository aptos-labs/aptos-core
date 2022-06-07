import * as SHA3 from 'js-sha3';
import { Buffer } from 'buffer/';
import {
  Ed25519PublicKey,
  Ed25519Signature,
  MultiEd25519PublicKey,
  MultiEd25519Signature,
  RawTransaction,
  SignedTransaction,
  TransactionAuthenticatorEd25519,
  TransactionAuthenticatorMultiEd25519,
} from './aptos_types';
import { bcsToBytes, Bytes } from './bcs';

const SALT = 'APTOS::RawTransaction';

export type SigningMessage = Buffer;

/** Function that takes in a Signing Message (serialized raw transaction)
 *  and returns a signature
 */
export type SigningFn = (txn: SigningMessage) => Ed25519Signature | MultiEd25519Signature;

class TransactionBuilder<F extends SigningFn> {
  protected readonly signingFunction: F;

  constructor(signingFunction: F) {
    this.signingFunction = signingFunction;
  }

  /** Generates a Signing Message out of a raw transaction. */
  static getSigningMessage(rawTxn: RawTransaction): SigningMessage {
    const hash = SHA3.sha3_256.create();
    hash.update(Buffer.from(SALT));

    const prefix = new Uint8Array(hash.arrayBuffer());

    return Buffer.from([...prefix, ...bcsToBytes(rawTxn)]);
  }
}

export class TransactionBuilderEd25519 extends TransactionBuilder<SigningFn> {
  private readonly publicKey: Uint8Array;

  constructor(signingFunction: SigningFn, publicKey: Uint8Array) {
    super(signingFunction);
    this.publicKey = publicKey;
  }

  private signInternal(rawTxn: RawTransaction): SignedTransaction {
    const signingMessage = TransactionBuilder.getSigningMessage(rawTxn);
    const signature = this.signingFunction(signingMessage);

    const authenticator = new TransactionAuthenticatorEd25519(
      new Ed25519PublicKey(this.publicKey),
      signature as Ed25519Signature,
    );

    return new SignedTransaction(rawTxn, authenticator);
  }

  /** Signs a raw transaction and returns a bcs serialized transaction. */
  sign(rawTxn: RawTransaction): Bytes {
    return bcsToBytes(this.signInternal(rawTxn));
  }
}

export class TransactionBuilderMultiEd25519 extends TransactionBuilder<SigningFn> {
  private readonly publicKey: MultiEd25519PublicKey;

  constructor(signingFunction: SigningFn, publicKey: MultiEd25519PublicKey) {
    super(signingFunction);
    this.publicKey = publicKey;
  }

  private signInternal(rawTxn: RawTransaction): SignedTransaction {
    const signingMessage = TransactionBuilder.getSigningMessage(rawTxn);
    const signature = this.signingFunction(signingMessage);

    const authenticator = new TransactionAuthenticatorMultiEd25519(this.publicKey, signature as MultiEd25519Signature);

    return new SignedTransaction(rawTxn, authenticator);
  }

  /** Signs a raw transaction and returns a bcs serialized transaction. */
  sign(rawTxn: RawTransaction): Bytes {
    return bcsToBytes(this.signInternal(rawTxn));
  }
}

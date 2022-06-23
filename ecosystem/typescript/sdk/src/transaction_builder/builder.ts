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
  SigningMessage,
  MultiAgentRawTransaction,
} from './aptos_types';
import { bcsToBytes, Bytes } from './bcs';

const RAW_TRANSACTION_SALT = 'APTOS::RawTransaction';
const RAW_TRANSACTION_WITH_DATA_SALT = 'APTOS::RawTransactionWithData';

type AnyRawTransaction = RawTransaction | MultiAgentRawTransaction;

/**
 * Function that takes in a Signing Message (serialized raw transaction)
 *  and returns a signature
 */
export type SigningFn = (txn: SigningMessage) => Ed25519Signature | MultiEd25519Signature;

export class TransactionBuilder<F extends SigningFn> {
  protected readonly signingFunction: F;

  constructor(signingFunction: F) {
    this.signingFunction = signingFunction;
  }

  /** Generates a Signing Message out of a raw transaction. */
  static getSigningMessage(rawTxn: AnyRawTransaction): SigningMessage {
    const hash = SHA3.sha3_256.create();
    if (rawTxn instanceof RawTransaction) {
      hash.update(Buffer.from(RAW_TRANSACTION_SALT));
    } else if (rawTxn instanceof MultiAgentRawTransaction) {
      hash.update(Buffer.from(RAW_TRANSACTION_WITH_DATA_SALT));
    } else {
      throw new Error('Unknown transaction type.');
    }

    const prefix = new Uint8Array(hash.arrayBuffer());

    return Buffer.from([...prefix, ...bcsToBytes(rawTxn)]);
  }
}

/**
 * Provides signing method for signing a raw transaction with single public key.
 */
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

/**
 * Provides signing method for signing a raw transaction with multisig public key.
 */
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

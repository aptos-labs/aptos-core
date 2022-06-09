import * as SHA3 from 'js-sha3';
import { HexString } from '../../hex_string';
import { Bytes } from '../bcs';
import { MultiEd25519PublicKey } from './multi_ed25519';

/**
 * Each account stores an authentication key. Authentication key enables account owners to rotate
 * their private key(s) associated with the account without changing the address that hosts their account.
 * @see {@link * https://aptos.dev/basics/basics-accounts | Account Basics}
 *
 * Account addresses can be derived from AuthenticationKey
 */
export class AuthenticationKey {
  static readonly LENGTH: number = 32;

  static readonly MULTI_ED25519_SCHEME: number = 1;

  readonly bytes: Bytes;

  constructor(bytes: Bytes) {
    if (bytes.length !== AuthenticationKey.LENGTH) {
      throw new Error('Expected a byte array of length 32');
    }
    this.bytes = bytes;
  }

  /**
   * Converts a K-of-N MultiEd25519PublicKey to AuthenticationKey with:
   * `auth_key = sha3-256(p_1 | … | p_n | K | 0x01)`. `K` represents the K-of-N required for
   * authenticating the transaction. `0x01` is the 1-byte scheme for multisig.
   */
  static fromMultiEd25519PublicKey(publicKey: MultiEd25519PublicKey): AuthenticationKey {
    const bytes = new Uint8Array([...publicKey.toBytes(), AuthenticationKey.MULTI_ED25519_SCHEME]);
    const hash = SHA3.sha3_256.create();
    hash.update(Buffer.from(bytes));

    return new AuthenticationKey(new Uint8Array(hash.arrayBuffer()));
  }

  /**
   * Derives an account address from AuthenticationKey. Since current AccountAddress is 32 bytes,
   * AuthenticationKey bytes are directly translated to AccountAddress.
   */
  derivedAddress(): HexString {
    return HexString.fromUint8Array(this.bytes);
  }
}

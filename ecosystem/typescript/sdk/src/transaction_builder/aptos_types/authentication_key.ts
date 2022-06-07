import * as SHA3 from 'js-sha3';
import { HexString } from '../../hex_string';
import { Bytes } from '../bcs';
import { AccountAddress } from './account_address';
import { MultiEd25519PublicKey } from './authenticator';

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

  static fromMultiEd25519PublicKey(publicKey: MultiEd25519PublicKey): AuthenticationKey {
    const bytes = new Uint8Array([...publicKey.toBytes(), AuthenticationKey.MULTI_ED25519_SCHEME]);
    const hash = SHA3.sha3_256.create();
    hash.update(Buffer.from(bytes));

    return new AuthenticationKey(new Uint8Array(hash.arrayBuffer()));
  }

  derivedAddress(): HexString {
    return HexString.fromUint8Array(this.bytes.subarray(AuthenticationKey.LENGTH - AccountAddress.LENGTH));
  }
}

/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';

/**
 * A single Secp256k1Ecdsa signature
 */
export type Secp256k1EcdsaSignature = {
    public_key: HexEncodedBytes;
    signature: HexEncodedBytes;
};


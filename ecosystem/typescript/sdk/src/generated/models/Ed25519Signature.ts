/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';

/**
 * A single Ed25519 signature
 */
export type Ed25519Signature = {
    public_key: HexEncodedBytes;
    signature: HexEncodedBytes;
};


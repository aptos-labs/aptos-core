/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';

export type MultiEd25519Signature = {
    public_keys: Array<HexEncodedBytes>;
    signatures: Array<HexEncodedBytes>;
    threshold: number;
    bitmap: HexEncodedBytes;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';
import type { U64 } from './U64';

/**
 * Account data
 *
 * A simplified version of the onchain Account resource
 */
export type AccountData = {
    sequence_number: U64;
    authentication_key: HexEncodedBytes;
};


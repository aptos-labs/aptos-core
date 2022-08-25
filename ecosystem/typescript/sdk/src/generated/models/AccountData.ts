/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes.js';
import type { U64 } from './U64.js';

export type AccountData = {
    sequence_number: U64;
    authentication_key: HexEncodedBytes;
};


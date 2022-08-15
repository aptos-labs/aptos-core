/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';
import type { U64 } from './U64';

export type AccountData = {
    sequence_number: U64;
    authentication_key: HexEncodedBytes;
};


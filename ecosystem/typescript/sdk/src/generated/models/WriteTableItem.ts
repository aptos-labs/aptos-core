/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { DecodedTableData } from './DecodedTableData.js';
import type { HexEncodedBytes } from './HexEncodedBytes.js';

export type WriteTableItem = {
    state_key_hash: string;
    handle: HexEncodedBytes;
    key: HexEncodedBytes;
    value: HexEncodedBytes;
    data?: DecodedTableData;
};


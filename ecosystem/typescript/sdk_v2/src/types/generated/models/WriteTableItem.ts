/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { DecodedTableData } from './DecodedTableData';
import type { HexEncodedBytes } from './HexEncodedBytes';

/**
 * Change set to write a table item
 */
export type WriteTableItem = {
    state_key_hash: string;
    handle: HexEncodedBytes;
    key: HexEncodedBytes;
    value: HexEncodedBytes;
    data?: DecodedTableData;
};


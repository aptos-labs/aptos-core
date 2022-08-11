/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { DecodedTableData } from './DecodedTableData';
import type { HexEncodedBytes } from './HexEncodedBytes';

export type WriteTableItem = {
    state_key_hash: string;
    handle: HexEncodedBytes;
    key: HexEncodedBytes;
    value: HexEncodedBytes;
    data?: DecodedTableData;
};


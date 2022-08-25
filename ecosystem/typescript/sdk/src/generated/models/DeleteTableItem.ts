/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { DeletedTableData } from './DeletedTableData.js';
import type { HexEncodedBytes } from './HexEncodedBytes.js';

export type DeleteTableItem = {
    state_key_hash: string;
    handle: HexEncodedBytes;
    key: HexEncodedBytes;
    data?: DeletedTableData;
};


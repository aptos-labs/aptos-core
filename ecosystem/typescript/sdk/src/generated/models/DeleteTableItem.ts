/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { DeletedTableData } from './DeletedTableData';
import type { HexEncodedBytes } from './HexEncodedBytes';

export type DeleteTableItem = {
    state_key_hash: string;
    handle: HexEncodedBytes;
    key: HexEncodedBytes;
    data?: DeletedTableData;
};


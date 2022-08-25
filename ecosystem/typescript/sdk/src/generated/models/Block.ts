/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HashValue } from './HashValue.js';
import type { Transaction } from './Transaction.js';
import type { U64 } from './U64.js';

export type Block = {
    block_height: U64;
    block_hash: HashValue;
    block_timestamp: U64;
    first_version: U64;
    last_version: U64;
    transactions?: Array<Transaction>;
};


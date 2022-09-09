/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { HashValue } from './HashValue';
import type { TransactionPayload } from './TransactionPayload';
import type { TransactionSignature } from './TransactionSignature';
import type { U64 } from './U64';

/**
 * A transaction waiting in mempool
 */
export type PendingTransaction = {
    hash: HashValue;
    sender: Address;
    sequence_number: U64;
    max_gas_amount: U64;
    gas_unit_price: U64;
    expiration_timestamp_secs: U64;
    payload: TransactionPayload;
    signature?: TransactionSignature;
};


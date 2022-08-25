/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { TransactionPayload } from './TransactionPayload.js';
import type { U64 } from './U64.js';

export type EncodeSubmissionRequest = {
    sender: Address;
    sequence_number: U64;
    max_gas_amount: U64;
    gas_unit_price: U64;
    expiration_timestamp_secs: U64;
    payload: TransactionPayload;
    secondary_signers?: Array<Address>;
};


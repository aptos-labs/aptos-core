/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { TransactionPayload } from './TransactionPayload';
import type { U64 } from './U64';

/**
 * Request to encode a submission
 */
export type EncodeSubmissionRequest = {
    sender: Address;
    sequence_number: U64;
    max_gas_amount: U64;
    gas_unit_price: U64;
    expiration_timestamp_secs: U64;
    payload: TransactionPayload;
    /**
     * Secondary signer accounts of the request for Multi-agent
     */
    secondary_signers?: Array<Address>;
};


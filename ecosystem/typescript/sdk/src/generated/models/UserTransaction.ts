/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { Event } from './Event';
import type { HashValue } from './HashValue';
import type { TransactionPayload } from './TransactionPayload';
import type { TransactionSignature } from './TransactionSignature';
import type { U64 } from './U64';
import type { WriteSetChange } from './WriteSetChange';

export type UserTransaction = {
    version: U64;
    hash: HashValue;
    state_root_hash: HashValue;
    event_root_hash: HashValue;
    gas_used: U64;
    success: boolean;
    vm_status: string;
    accumulator_root_hash: HashValue;
    changes: Array<WriteSetChange>;
    sender: Address;
    sequence_number: U64;
    max_gas_amount: U64;
    gas_unit_price: U64;
    expiration_timestamp_secs: U64;
    payload: TransactionPayload;
    signature?: TransactionSignature;
    events: Array<Event>;
    timestamp: U64;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { Event } from './Event.js';
import type { HashValue } from './HashValue.js';
import type { TransactionPayload } from './TransactionPayload.js';
import type { TransactionSignature } from './TransactionSignature.js';
import type { U64 } from './U64.js';
import type { WriteSetChange } from './WriteSetChange.js';

export type UserTransaction = {
    version: U64;
    hash: HashValue;
    state_change_hash: HashValue;
    event_root_hash: HashValue;
    state_checkpoint_hash?: HashValue;
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


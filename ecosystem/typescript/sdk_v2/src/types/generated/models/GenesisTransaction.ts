/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Event } from './Event';
import type { GenesisPayload } from './GenesisPayload';
import type { HashValue } from './HashValue';
import type { U64 } from './U64';
import type { WriteSetChange } from './WriteSetChange';

/**
 * The genesis transaction
 *
 * This only occurs at the genesis transaction (version 0)
 */
export type GenesisTransaction = {
    version: U64;
    hash: HashValue;
    state_change_hash: HashValue;
    event_root_hash: HashValue;
    state_checkpoint_hash?: HashValue;
    gas_used: U64;
    /**
     * Whether the transaction was successful
     */
    success: boolean;
    /**
     * The VM status of the transaction, can tell useful information in a failure
     */
    vm_status: string;
    accumulator_root_hash: HashValue;
    /**
     * Final state of resources changed by the transaction
     */
    changes: Array<WriteSetChange>;
    payload: GenesisPayload;
    /**
     * Events emitted during genesis
     */
    events: Array<Event>;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HashValue } from './HashValue';
import type { U64 } from './U64';
import type { WriteSetChange } from './WriteSetChange';

/**
 * A state checkpoint transaction
 */
export type StateCheckpointTransaction = {
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
    timestamp: U64;
};


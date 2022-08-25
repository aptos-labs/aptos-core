/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HashValue } from './HashValue.js';
import type { U64 } from './U64.js';
import type { WriteSetChange } from './WriteSetChange.js';

export type StateCheckpointTransaction = {
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
    timestamp: U64;
};


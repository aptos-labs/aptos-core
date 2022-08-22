/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { Event } from './Event';
import type { HashValue } from './HashValue';
import type { U64 } from './U64';
import type { WriteSetChange } from './WriteSetChange';

export type BlockMetadataTransaction = {
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
    id: HashValue;
    epoch: U64;
    round: U64;
    events: Array<Event>;
    previous_block_votes_bitvec: Array<number>;
    proposer: Address;
    failed_proposer_indices: Array<number>;
    timestamp: U64;
};


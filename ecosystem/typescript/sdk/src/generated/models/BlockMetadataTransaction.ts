/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { Event } from './Event';
import type { HashValue } from './HashValue';
import type { U64 } from './U64';
import type { WriteSetChange } from './WriteSetChange';

/**
 * A block metadata transaction
 *
 * This signifies the beginning of a block, and contains information
 * about the specific block
 */
export type BlockMetadataTransaction = {
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
    id: HashValue;
    epoch: U64;
    round: U64;
    /**
     * The events emitted at the block creation
     */
    events: Array<Event>;
    /**
     * Previous block votes
     */
    previous_block_votes_bitvec: Array<number>;
    proposer: Address;
    /**
     * The indices of the proposers who failed to propose
     */
    failed_proposer_indices: Array<number>;
    timestamp: U64;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EventGuid } from './EventGuid';
import type { MoveType } from './MoveType';
import type { U64 } from './U64';

/**
 * An event from a transaction with a version
 */
export type VersionedEvent = {
    version: U64;
    guid: EventGuid;
    sequence_number: U64;
    type: MoveType;
    /**
     * The JSON representation of the event
     */
    data: any;
};


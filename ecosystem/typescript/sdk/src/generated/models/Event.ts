/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EventKeyWrapper } from './EventKeyWrapper';
import type { MoveType } from './MoveType';
import type { U64 } from './U64';

/**
 * An event from a transaction
 */
export type Event = {
    guid: EventKeyWrapper;
    sequence_number: U64;
    type: MoveType;
    /**
     * The JSON representation of the event
     */
    data: any;
};


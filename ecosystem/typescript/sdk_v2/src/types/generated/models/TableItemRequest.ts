/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveType } from './MoveType';

/**
 * Table Item request for the GetTableItem API
 */
export type TableItemRequest = {
    key_type: MoveType;
    value_type: MoveType;
    /**
     * The value of the table item's key
     */
    key: any;
};


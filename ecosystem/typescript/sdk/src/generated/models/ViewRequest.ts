/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EntryFunctionId } from './EntryFunctionId';
import type { MoveType } from './MoveType';

/**
 * View request for the Move View Function API
 */
export type ViewRequest = {
    function: EntryFunctionId;
    /**
     * Type arguments of the function
     */
    type_arguments: Array<MoveType>;
    /**
     * Arguments of the function
     */
    arguments: Array<any>;
};


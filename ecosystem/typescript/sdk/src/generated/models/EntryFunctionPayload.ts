/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EntryFunctionId } from './EntryFunctionId';
import type { MoveType } from './MoveType';

/**
 * Payload which runs a single entry function
 */
export type EntryFunctionPayload = {
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


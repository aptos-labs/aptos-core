/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EntryFunctionId } from './EntryFunctionId.js';
import type { MoveType } from './MoveType.js';

export type EntryFunctionPayload = {
    function: EntryFunctionId;
    type_arguments: Array<MoveType>;
    arguments: Array<any>;
};


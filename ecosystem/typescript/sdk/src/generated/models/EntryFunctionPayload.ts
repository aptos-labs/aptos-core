/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EntryFunctionId } from './EntryFunctionId';
import type { MoveType } from './MoveType';

export type EntryFunctionPayload = {
    function: EntryFunctionId;
    type_arguments: Array<MoveType>;
    arguments: Array<any>;
};


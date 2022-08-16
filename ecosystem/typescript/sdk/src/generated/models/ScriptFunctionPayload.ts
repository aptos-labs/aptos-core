/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveType } from './MoveType';
import type { EntryFunctionId } from './EntryFunctionId';

export type EntryFunctionPayload = {
    function: EntryFunctionId;
    type_arguments: Array<MoveType>;
    arguments: Array<any>;
};


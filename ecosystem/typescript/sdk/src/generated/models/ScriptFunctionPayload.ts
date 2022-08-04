/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveType } from './MoveType';
import type { ScriptFunctionId } from './ScriptFunctionId';

export type ScriptFunctionPayload = {
    function: ScriptFunctionId;
    type_arguments: Array<MoveType>;
    arguments: Array<any>;
};


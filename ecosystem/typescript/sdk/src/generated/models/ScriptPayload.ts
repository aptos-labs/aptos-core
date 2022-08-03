/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveScriptBytecode } from './MoveScriptBytecode';
import type { MoveType } from './MoveType';

export type ScriptPayload = {
    code: MoveScriptBytecode;
    type_arguments: Array<MoveType>;
    arguments: Array<any>;
};


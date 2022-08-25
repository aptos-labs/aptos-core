/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveScriptBytecode } from './MoveScriptBytecode.js';
import type { MoveType } from './MoveType.js';

export type ScriptPayload = {
    code: MoveScriptBytecode;
    type_arguments: Array<MoveType>;
    arguments: Array<any>;
};


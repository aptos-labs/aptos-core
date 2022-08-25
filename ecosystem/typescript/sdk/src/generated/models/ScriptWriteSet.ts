/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { ScriptPayload } from './ScriptPayload.js';

export type ScriptWriteSet = {
    execute_as: Address;
    script: ScriptPayload;
};


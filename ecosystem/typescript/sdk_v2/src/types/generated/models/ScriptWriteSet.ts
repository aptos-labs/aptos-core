/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { ScriptPayload } from './ScriptPayload';

export type ScriptWriteSet = {
    execute_as: Address;
    script: ScriptPayload;
};


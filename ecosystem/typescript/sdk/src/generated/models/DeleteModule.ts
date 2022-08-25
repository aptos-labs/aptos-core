/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { MoveModuleId } from './MoveModuleId.js';

export type DeleteModule = {
    address: Address;
    state_key_hash: string;
    module: MoveModuleId;
};


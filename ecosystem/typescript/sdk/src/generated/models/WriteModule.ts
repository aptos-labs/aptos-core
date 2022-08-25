/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { MoveModuleBytecode } from './MoveModuleBytecode.js';

export type WriteModule = {
    address: Address;
    state_key_hash: string;
    data: MoveModuleBytecode;
};


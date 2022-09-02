/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { MoveModuleBytecode } from './MoveModuleBytecode';

/**
 * Write a new module or update an existing one
 */
export type WriteModule = {
    address: Address;
    state_key_hash: string;
    data: MoveModuleBytecode;
};


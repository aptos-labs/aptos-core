/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { MoveModuleId } from './MoveModuleId';

/**
 * Delete a module
 */
export type DeleteModule = {
    address: Address;
    state_key_hash: string;
    module: MoveModuleId;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { IdentifierWrapper } from './IdentifierWrapper';
import type { MoveFunction } from './MoveFunction';
import type { MoveModuleId } from './MoveModuleId';
import type { MoveStruct } from './MoveStruct';

/**
 * A Move module
 */
export type MoveModule = {
    address: Address;
    name: IdentifierWrapper;
    /**
     * Friends of the module
     */
    friends: Array<MoveModuleId>;
    /**
     * Public functions of the module
     */
    exposed_functions: Array<MoveFunction>;
    /**
     * Structs of the module
     */
    structs: Array<MoveStruct>;
};


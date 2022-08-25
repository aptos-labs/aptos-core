/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { IdentifierWrapper } from './IdentifierWrapper.js';
import type { MoveFunction } from './MoveFunction.js';
import type { MoveModuleId } from './MoveModuleId.js';
import type { MoveStruct } from './MoveStruct.js';

export type MoveModule = {
    address: Address;
    name: IdentifierWrapper;
    friends: Array<MoveModuleId>;
    exposed_functions: Array<MoveFunction>;
    structs: Array<MoveStruct>;
};


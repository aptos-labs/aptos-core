/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { IdentifierWrapper } from './IdentifierWrapper';
import type { MoveFunction } from './MoveFunction';
import type { MoveModuleId } from './MoveModuleId';
import type { MoveStruct } from './MoveStruct';

export type MoveModule = {
    address: Address;
    name: IdentifierWrapper;
    friends: Array<MoveModuleId>;
    exposed_functions: Array<MoveFunction>;
    structs: Array<MoveStruct>;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { IdentifierWrapper } from './IdentifierWrapper';
import type { MoveType } from './MoveType';

export type MoveStructTag = {
    address: Address;
    module: IdentifierWrapper;
    name: IdentifierWrapper;
    generic_type_params: Array<MoveType>;
};


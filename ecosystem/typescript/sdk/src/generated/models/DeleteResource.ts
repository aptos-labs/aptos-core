/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { MoveStructTag } from './MoveStructTag';

export type DeleteResource = {
    address: Address;
    state_key_hash: string;
    resource: MoveStructTag;
};


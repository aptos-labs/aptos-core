/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { MoveStructTag } from './MoveStructTag.js';

export type DeleteResource = {
    address: Address;
    state_key_hash: string;
    resource: MoveStructTag;
};


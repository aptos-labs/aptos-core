/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { MoveResource } from './MoveResource.js';

export type WriteResource = {
    address: Address;
    state_key_hash: string;
    data: MoveResource;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { MoveResource } from './MoveResource';

export type WriteResource = {
    address: Address;
    state_key_hash: string;
    data: MoveResource;
};


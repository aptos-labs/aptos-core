/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveStructTag } from './MoveStructTag';
import type { MoveStructValue } from './MoveStructValue';

/**
 * A parsed Move resource
 */
export type MoveResource = {
    type: MoveStructTag;
    data: MoveStructValue;
};


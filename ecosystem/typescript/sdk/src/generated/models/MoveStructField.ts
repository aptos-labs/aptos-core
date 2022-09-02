/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { IdentifierWrapper } from './IdentifierWrapper';
import type { MoveType } from './MoveType';

/**
 * Move struct field
 */
export type MoveStructField = {
    name: IdentifierWrapper;
    type: MoveType;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address';
import type { HexEncodedBytes } from './HexEncodedBytes';
import type { MoveStructValue } from './MoveStructValue';
import type { U128 } from './U128';
import type { U256 } from './U256';
import type { U64 } from './U64';

/**
 * An enum of the possible Move value types
 */
export type MoveValue = (number | U64 | U128 | U256 | boolean | Address | Array<MoveValue> | HexEncodedBytes | MoveStructValue | string);


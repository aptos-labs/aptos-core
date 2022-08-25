/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Address } from './Address.js';
import type { HexEncodedBytes } from './HexEncodedBytes.js';
import type { MoveStructValue } from './MoveStructValue.js';
import type { U128 } from './U128.js';
import type { U64 } from './U64.js';

export type MoveValue = (number | U64 | U128 | boolean | Address | Array<MoveValue> | HexEncodedBytes | MoveStructValue | string);


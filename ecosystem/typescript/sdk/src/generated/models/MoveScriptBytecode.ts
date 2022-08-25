/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes.js';
import type { MoveFunction } from './MoveFunction.js';

export type MoveScriptBytecode = {
    bytecode: HexEncodedBytes;
    abi?: MoveFunction;
};


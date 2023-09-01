/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';
import type { MoveFunction } from './MoveFunction';

/**
 * Move script bytecode
 */
export type MoveScriptBytecode = {
    bytecode: HexEncodedBytes;
    abi?: MoveFunction;
};


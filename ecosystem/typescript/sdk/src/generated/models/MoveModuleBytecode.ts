/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';
import type { MoveModule } from './MoveModule';

/**
 * Move module bytecode along with it's ABI
 */
export type MoveModuleBytecode = {
    bytecode: HexEncodedBytes;
    abi?: MoveModule;
};


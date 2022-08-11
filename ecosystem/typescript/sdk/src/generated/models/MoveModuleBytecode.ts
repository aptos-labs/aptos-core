/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes';
import type { MoveModule } from './MoveModule';

export type MoveModuleBytecode = {
    bytecode: HexEncodedBytes;
    abi?: MoveModule;
};


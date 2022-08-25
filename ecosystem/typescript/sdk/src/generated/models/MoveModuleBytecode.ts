/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { HexEncodedBytes } from './HexEncodedBytes.js';
import type { MoveModule } from './MoveModule.js';

export type MoveModuleBytecode = {
    bytecode: HexEncodedBytes;
    abi?: MoveModule;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { IdentifierWrapper } from './IdentifierWrapper.js';
import type { MoveFunctionGenericTypeParam } from './MoveFunctionGenericTypeParam.js';
import type { MoveFunctionVisibility } from './MoveFunctionVisibility.js';
import type { MoveType } from './MoveType.js';

export type MoveFunction = {
    name: IdentifierWrapper;
    visibility: MoveFunctionVisibility;
    is_entry: boolean;
    generic_type_params: Array<MoveFunctionGenericTypeParam>;
    params: Array<MoveType>;
    return: Array<MoveType>;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { IdentifierWrapper } from './IdentifierWrapper';
import type { MoveFunctionGenericTypeParam } from './MoveFunctionGenericTypeParam';
import type { MoveFunctionVisibility } from './MoveFunctionVisibility';
import type { MoveType } from './MoveType';

/**
 * Move function
 */
export type MoveFunction = {
    name: IdentifierWrapper;
    visibility: MoveFunctionVisibility;
    /**
     * Whether the function can be called as an entry function directly in a transaction
     */
    is_entry: boolean;
    /**
     * Whether the function is a view function or not
     */
    is_view: boolean;
    /**
     * Generic type params associated with the Move function
     */
    generic_type_params: Array<MoveFunctionGenericTypeParam>;
    /**
     * Parameters associated with the move function
     */
    params: Array<MoveType>;
    /**
     * Return type of the function
     */
    return: Array<MoveType>;
};


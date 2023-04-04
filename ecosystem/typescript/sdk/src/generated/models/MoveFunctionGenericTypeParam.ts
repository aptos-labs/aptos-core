/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveAbility } from './MoveAbility';

/**
 * Move function generic type param
 */
export type MoveFunctionGenericTypeParam = {
    /**
     * Move abilities tied to the generic type param and associated with the function that uses it
     */
    constraints: Array<MoveAbility>;
};


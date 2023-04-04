/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { MoveAbility } from './MoveAbility';

/**
 * Move generic type param
 */
export type MoveStructGenericTypeParam = {
    /**
     * Move abilities tied to the generic type param and associated with the type that uses it
     */
    constraints: Array<MoveAbility>;
};


/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { IdentifierWrapper } from './IdentifierWrapper.js';
import type { MoveAbility } from './MoveAbility.js';
import type { MoveStructField } from './MoveStructField.js';
import type { MoveStructGenericTypeParam } from './MoveStructGenericTypeParam.js';

export type MoveStruct = {
    name: IdentifierWrapper;
    is_native: boolean;
    abilities: Array<MoveAbility>;
    generic_type_params: Array<MoveStructGenericTypeParam>;
    fields: Array<MoveStructField>;
};


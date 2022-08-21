/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AptosErrorCode } from './AptosErrorCode';

/**
 * This is the generic struct we use for all API errors, it contains a string
 * message and an Aptos API specific error code.
 */
export type AptosError = {
    message: string;
    error_code: AptosErrorCode;
    vm_error_code?: number;
};


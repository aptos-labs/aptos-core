/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * These codes provide more granular error information beyond just the HTTP
 * status code of the response.
 */
export enum AptosErrorCode {
    READ_FROM_STORAGE_ERROR = 'read_from_storage_error',
    INVALID_BCS_IN_STORAGE_ERROR = 'invalid_bcs_in_storage_error',
    BCS_SERIALIZATION_ERROR = 'bcs_serialization_error',
    INVALID_START_PARAM = 'invalid_start_param',
    INVALID_LIMIT_PARAM = 'invalid_limit_param',
}

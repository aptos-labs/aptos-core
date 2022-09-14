/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * These codes provide more granular error information beyond just the HTTP
 * status code of the response.
 */
export enum AptosErrorCode {
    ACCOUNT_NOT_FOUND = 'account_not_found',
    RESOURCE_NOT_FOUND = 'resource_not_found',
    MODULE_NOT_FOUND = 'module_not_found',
    STRUCT_FIELD_NOT_FOUND = 'struct_field_not_found',
    VERSION_NOT_FOUND = 'version_not_found',
    TRANSACTION_NOT_FOUND = 'transaction_not_found',
    TABLE_ITEM_NOT_FOUND = 'table_item_not_found',
    BLOCK_NOT_FOUND = 'block_not_found',
    VERSION_PRUNED = 'version_pruned',
    BLOCK_PRUNED = 'block_pruned',
    INVALID_INPUT = 'invalid_input',
    INVALID_TRANSACTION_UPDATE = 'invalid_transaction_update',
    SEQUENCE_NUMBER_TOO_OLD = 'sequence_number_too_old',
    VM_ERROR = 'vm_error',
    HEALTH_CHECK_FAILED = 'health_check_failed',
    MEMPOOL_IS_FULL = 'mempool_is_full',
    INTERNAL_ERROR = 'internal_error',
    WEB_FRAMEWORK_ERROR = 'web_framework_error',
    BCS_NOT_SUPPORTED = 'bcs_not_supported',
    API_DISABLED = 'api_disabled',
}

/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

/**
 * These codes provide more granular error information beyond just the HTTP
 * status code of the response.
 */
export enum VelorTapErrorCode {
    YEAH_NAH_YEAH_YEAH_YEAH_NAH_YEAH_NAH = 'YeahNahYeahYeahYeahNahYeahNah',
    INVALID_REQUEST = 'InvalidRequest',
    ACCOUNT_DOES_NOT_EXIST = 'AccountDoesNotExist',
    REJECTED = 'Rejected',
    SOURCE_IP_MISSING = 'SourceIpMissing',
    TRANSACTION_FAILED = 'TransactionFailed',
    ENDPOINT_NOT_ENABLED = 'EndpointNotEnabled',
    VELOR_API_ERROR = 'VelorApiError',
    BYPASSER_ERROR = 'BypasserError',
    CHECKER_ERROR = 'CheckerError',
    STORAGE_ERROR = 'StorageError',
    FUNDER_ACCOUNT_PROBLEM = 'FunderAccountProblem',
    TRANSACTION_TIMED_OUT = 'TransactionTimedOut',
    SERIALIZATION_ERROR = 'SerializationError',
    SERVER_OVERLOADED = 'ServerOverloaded',
    WEB_FRAMEWORK_ERROR = 'WebFrameworkError',
}

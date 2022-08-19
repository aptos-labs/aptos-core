/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AptosError } from './AptosError';

export type TransactionsBatchSingleSubmissionFailure = {
    error: AptosError;
    transaction_index: number;
};


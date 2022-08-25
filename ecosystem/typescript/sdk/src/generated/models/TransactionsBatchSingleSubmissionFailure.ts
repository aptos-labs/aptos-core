/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { AptosError } from './AptosError.js';

export type TransactionsBatchSingleSubmissionFailure = {
    error: AptosError;
    transaction_index: number;
};


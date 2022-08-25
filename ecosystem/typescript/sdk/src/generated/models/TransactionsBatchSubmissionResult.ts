/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { TransactionsBatchSingleSubmissionFailure } from './TransactionsBatchSingleSubmissionFailure.js';

export type TransactionsBatchSubmissionResult = {
    transaction_failures: Array<TransactionsBatchSingleSubmissionFailure>;
};


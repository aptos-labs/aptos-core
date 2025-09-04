/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { VelorTapErrorCode } from './VelorTapErrorCode';
import type { RejectionReason } from './RejectionReason';

/**
 * This is the generic struct we use for all API errors, it contains a string
 * message and a service specific error code.
 */
export type VelorTapError = {
    /**
     * A message describing the error
     */
    message: string;
    error_code: VelorTapErrorCode;
    /**
     * If we're returning a 403 because we're rejecting the mint request, this
     * contains additional reasons why.
     */
    rejection_reasons: Array<RejectionReason>;
    /**
     * Submitted transaction hashes, if it got to that point.
     */
    txn_hashes: Array<string>;
};


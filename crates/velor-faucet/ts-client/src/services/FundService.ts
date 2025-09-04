/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { VelorTapError } from '../models/VelorTapError';
import type { FundRequest } from '../models/FundRequest';
import type { FundResponse } from '../models/FundResponse';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class FundService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Funds an account
     * With this endpoint a user can create and fund an account. Depending on
     * the configured funding backend, this may do different things under the
     * hood (e.g. minting vs transferring) and have different funding semantics
     * (e.g. whether it will fund an account if it already exists).
     * @returns FundResponse
     * @returns VelorTapError
     * @throws ApiError
     */
    public fund({
        requestBody,
    }: {
        requestBody: FundRequest,
    }): CancelablePromise<FundResponse | VelorTapError> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/fund',
            body: requestBody,
            mediaType: 'application/json',
        });
    }

    /**
     * Check whether a given requester is eligible to be funded
     * This function runs only the various eligibility checks that we perform
     * in `fund` without actually funding the account or writing anything to
     * storage. If the request is valid it returns an empty 200. If it is invalid
     * it returns a 400 or 403 with an explanation in the response body.
     * @returns any
     * @returns VelorTapError
     * @throws ApiError
     */
    public isEligible({
        requestBody,
    }: {
        requestBody: FundRequest,
    }): CancelablePromise<any | VelorTapError> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/is_eligible',
            body: requestBody,
            mediaType: 'application/json',
        });
    }

}

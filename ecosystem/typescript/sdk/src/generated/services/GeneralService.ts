/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { IndexResponse } from '../models/IndexResponse';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class GeneralService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Show OpenAPI explorer
     * Provides a UI that you can use to explore the API. You can also retrieve the API directly at `/spec.yaml` and `/spec.json`.
     * @returns string
     * @throws ApiError
     */
    public spec(): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/spec',
        });
    }

    /**
     * Get ledger info
     * Get the latest ledger information, including data such as chain ID,
     * role type, ledger versions, epoch, etc.
     * @returns IndexResponse
     * @throws ApiError
     */
    public getLedgerInfo(): CancelablePromise<IndexResponse> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/',
        });
    }

}

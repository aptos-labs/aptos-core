/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class GeneralService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Show OpenAPI explorer
     * Provides a UI that you can use to explore the API. You can also
     * retrieve the API directly at `/spec.yaml` and `/spec.json`.
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
     * Check API health
     * Basic endpoint that always returns Ok for health.
     * @returns string
     * @throws ApiError
     */
    public root(): CancelablePromise<string> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/',
        });
    }

}

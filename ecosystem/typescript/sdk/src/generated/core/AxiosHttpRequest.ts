/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiRequestOptions } from './ApiRequestOptions.js';
import { BaseHttpRequest } from './BaseHttpRequest.js';
import type { CancelablePromise } from './CancelablePromise.js';
import type { OpenAPIConfig } from './OpenAPI.js';
import { request as __request } from './request.js';

export class AxiosHttpRequest extends BaseHttpRequest {

    constructor(config: OpenAPIConfig) {
        super(config);
    }

    /**
     * Request method
     * @param options The request options from the service
     * @returns CancelablePromise<T>
     * @throws ApiError
     */
    public override request<T>(options: ApiRequestOptions): CancelablePromise<T> {
        return __request(this.config, options);
    }
}

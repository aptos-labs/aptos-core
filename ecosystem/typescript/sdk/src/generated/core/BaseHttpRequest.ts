/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiRequestOptions } from './ApiRequestOptions.js';
import type { CancelablePromise } from './CancelablePromise.js';
import type { OpenAPIConfig } from './OpenAPI.js';

export abstract class BaseHttpRequest {

    constructor(public readonly config: OpenAPIConfig) {}

    public abstract request<T>(options: ApiRequestOptions): CancelablePromise<T>;
}

/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { VelorTapError } from '../models/VelorTapError';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class CaptchaService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Initiate captcha flow
     * With this endpoint you can initiate a captcha flow. The response will
     * contain an image (the captcha to solve) in the body and a code in the
     * header that you must include in the call to `/fund`. This endpoint is
     * only relevant if the CaptchaChecker is enabled.
     * @returns binary
     * @returns VelorTapError
     * @throws ApiError
     */
    public requestCaptcha(): CancelablePromise<Blob | VelorTapError> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/request_captcha',
        });
    }

}

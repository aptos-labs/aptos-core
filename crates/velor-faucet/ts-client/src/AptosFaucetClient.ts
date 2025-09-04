/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { BaseHttpRequest } from './core/BaseHttpRequest';
import type { OpenAPIConfig } from './core/OpenAPI';
import { AxiosHttpRequest } from './core/AxiosHttpRequest';

import { CaptchaService } from './services/CaptchaService';
import { FundService } from './services/FundService';
import { GeneralService } from './services/GeneralService';

type HttpRequestConstructor = new (config: OpenAPIConfig) => BaseHttpRequest;

export class VelorFaucetClient {

    public readonly captcha: CaptchaService;
    public readonly fund: FundService;
    public readonly general: GeneralService;

    public readonly request: BaseHttpRequest;

    constructor(config?: Partial<OpenAPIConfig>, HttpRequest: HttpRequestConstructor = AxiosHttpRequest) {
        this.request = new HttpRequest({
            BASE: config?.BASE ?? '/v1',
            VERSION: config?.VERSION ?? '0.1.0',
            WITH_CREDENTIALS: config?.WITH_CREDENTIALS ?? false,
            CREDENTIALS: config?.CREDENTIALS ?? 'include',
            TOKEN: config?.TOKEN,
            USERNAME: config?.USERNAME,
            PASSWORD: config?.PASSWORD,
            HEADERS: config?.HEADERS,
            ENCODE_PATH: config?.ENCODE_PATH,
        });

        this.captcha = new CaptchaService(this.request);
        this.fund = new FundService(this.request);
        this.general = new GeneralService(this.request);
    }
}


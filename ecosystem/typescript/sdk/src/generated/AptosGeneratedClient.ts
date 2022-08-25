/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { BaseHttpRequest } from './core/BaseHttpRequest.js';
import type { OpenAPIConfig } from './core/OpenAPI.js';
import { AxiosHttpRequest } from './core/AxiosHttpRequest.js';

import { AccountsService } from './services/AccountsService.js';
import { BlocksService } from './services/BlocksService.js';
import { EventsService } from './services/EventsService.js';
import { GeneralService } from './services/GeneralService.js';
import { TablesService } from './services/TablesService.js';
import { TransactionsService } from './services/TransactionsService.js';

type HttpRequestConstructor = new (config: OpenAPIConfig) => BaseHttpRequest;

export class AptosGeneratedClient {

    public readonly accounts: AccountsService;
    public readonly blocks: BlocksService;
    public readonly events: EventsService;
    public readonly general: GeneralService;
    public readonly tables: TablesService;
    public readonly transactions: TransactionsService;

    public readonly request: BaseHttpRequest;

    constructor(config?: Partial<OpenAPIConfig>, HttpRequest: HttpRequestConstructor = AxiosHttpRequest) {
        this.request = new HttpRequest({
            BASE: config?.BASE ?? '/v1',
            VERSION: config?.VERSION ?? '1.0.1',
            WITH_CREDENTIALS: config?.WITH_CREDENTIALS ?? false,
            CREDENTIALS: config?.CREDENTIALS ?? 'include',
            TOKEN: config?.TOKEN,
            USERNAME: config?.USERNAME,
            PASSWORD: config?.PASSWORD,
            HEADERS: config?.HEADERS,
            ENCODE_PATH: config?.ENCODE_PATH,
        });

        this.accounts = new AccountsService(this.request);
        this.blocks = new BlocksService(this.request);
        this.events = new EventsService(this.request);
        this.general = new GeneralService(this.request);
        this.tables = new TablesService(this.request);
        this.transactions = new TransactionsService(this.request);
    }
}


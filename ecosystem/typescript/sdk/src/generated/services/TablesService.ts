/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Address } from '../models/Address.js';
import type { MoveValue } from '../models/MoveValue.js';
import type { TableItemRequest } from '../models/TableItemRequest.js';
import type { U64 } from '../models/U64.js';

import type { CancelablePromise } from '../core/CancelablePromise.js';
import type { BaseHttpRequest } from '../core/BaseHttpRequest.js';

export class TablesService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get table item
     * Get a table item from the table identified by {table_handle} in the
     * path and the "key" (TableItemRequest) provided in the request body.
     *
     * This is a POST endpoint because the "key" for requesting a specific
     * table item (TableItemRequest) could be quite complex, as each of its
     * fields could themselves be composed of other structs. This makes it
     * impractical to express using query params, meaning GET isn't an option.
     * @param tableHandle
     * @param requestBody
     * @param ledgerVersion
     * @returns MoveValue
     * @throws ApiError
     */
    public getTableItem(
        tableHandle: Address,
        requestBody: TableItemRequest,
        ledgerVersion?: U64,
    ): CancelablePromise<MoveValue> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/tables/{table_handle}/item',
            path: {
                'table_handle': tableHandle,
            },
            query: {
                'ledger_version': ledgerVersion,
            },
            body: requestBody,
            mediaType: 'application/json',
        });
    }

}

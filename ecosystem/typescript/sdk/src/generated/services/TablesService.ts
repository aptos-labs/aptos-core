/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { MoveValue } from '../models/MoveValue';
import type { TableItemRequest } from '../models/TableItemRequest';
import type { U128 } from '../models/U128';
import type { U64 } from '../models/U64';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

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
        tableHandle: U128,
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

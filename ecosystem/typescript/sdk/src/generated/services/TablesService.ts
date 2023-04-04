/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Address } from '../models/Address';
import type { MoveValue } from '../models/MoveValue';
import type { RawTableItemRequest } from '../models/RawTableItemRequest';
import type { TableItemRequest } from '../models/TableItemRequest';
import type { U64 } from '../models/U64';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class TablesService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get table item
     * Get a table item at a specific ledger version from the table identified by {table_handle}
     * in the path and the "key" (TableItemRequest) provided in the request body.
     *
     * This is a POST endpoint because the "key" for requesting a specific
     * table item (TableItemRequest) could be quite complex, as each of its
     * fields could themselves be composed of other structs. This makes it
     * impractical to express using query params, meaning GET isn't an option.
     *
     * The Aptos nodes prune account state history, via a configurable time window.
     * If the requested ledger version has been pruned, the server responds with a 410.
     * @param tableHandle Table handle hex encoded 32-byte string
     * @param requestBody
     * @param ledgerVersion Ledger version to get state of account
     *
     * If not provided, it will be the latest version
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

    /**
     * Get raw table item
     * Get a table item at a specific ledger version from the table identified by {table_handle}
     * in the path and the "key" (RawTableItemRequest) provided in the request body.
     *
     * The `get_raw_table_item` requires only a serialized key comparing to the full move type information
     * comparing to the `get_table_item` api, and can only return the query in the bcs format.
     *
     * The Aptos nodes prune account state history, via a configurable time window.
     * If the requested ledger version has been pruned, the server responds with a 410.
     * @param tableHandle Table handle hex encoded 32-byte string
     * @param requestBody
     * @param ledgerVersion Ledger version to get state of account
     *
     * If not provided, it will be the latest version
     * @returns MoveValue
     * @throws ApiError
     */
    public getRawTableItem(
        tableHandle: Address,
        requestBody: RawTableItemRequest,
        ledgerVersion?: U64,
    ): CancelablePromise<MoveValue> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/tables/{table_handle}/raw_item',
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

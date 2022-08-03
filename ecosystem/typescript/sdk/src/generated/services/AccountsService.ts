/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { AccountData } from '../models/AccountData';
import type { Address } from '../models/Address';
import type { IdentifierWrapper } from '../models/IdentifierWrapper';
import type { MoveModuleBytecode } from '../models/MoveModuleBytecode';
import type { MoveResource } from '../models/MoveResource';
import type { MoveStructTagParam } from '../models/MoveStructTagParam';
import type { U64 } from '../models/U64';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class AccountsService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get account
     * Return high level information about an account such as its sequence number.
     * @param address
     * @param ledgerVersion
     * @returns AccountData
     * @throws ApiError
     */
    public getAccount(
        address: Address,
        ledgerVersion?: U64,
    ): CancelablePromise<AccountData> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}',
            path: {
                'address': address,
            },
            query: {
                'ledger_version': ledgerVersion,
            },
        });
    }

    /**
     * Get account resources
     * This endpoint returns all account resources at a given address at a
     * specific ledger version (AKA transaction version). If the ledger
     * version is not specified in the request, the latest ledger version is used.
     *
     * The Aptos nodes prune account state history, via a configurable time window (link).
     * If the requested data has been pruned, the server responds with a 404.
     * @param address
     * @param ledgerVersion
     * @returns MoveResource
     * @throws ApiError
     */
    public getAccountResources(
        address: Address,
        ledgerVersion?: U64,
    ): CancelablePromise<Array<MoveResource>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}/resources',
            path: {
                'address': address,
            },
            query: {
                'ledger_version': ledgerVersion,
            },
        });
    }

    /**
     * Get account modules
     * This endpoint returns all account modules at a given address at a
     * specific ledger version (AKA transaction version). If the ledger
     * version is not specified in the request, the latest ledger version is used.
     *
     * The Aptos nodes prune account state history, via a configurable time window (link).
     * If the requested data has been pruned, the server responds with a 404.
     * @param address
     * @param ledgerVersion
     * @returns MoveModuleBytecode
     * @throws ApiError
     */
    public getAccountModules(
        address: Address,
        ledgerVersion?: U64,
    ): CancelablePromise<Array<MoveModuleBytecode>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}/modules',
            path: {
                'address': address,
            },
            query: {
                'ledger_version': ledgerVersion,
            },
        });
    }

    /**
     * Get specific account resource
     * This endpoint returns the resource of a specific type residing at a given
     * account at a specified ledger version (AKA transaction version). If the
     * ledger version is not specified in the request, the latest ledger version
     * is used.
     *
     * The Aptos nodes prune account state history, via a configurable time window (link).
     * If the requested data has been pruned, the server responds with a 404.
     * @param address
     * @param resourceType
     * @param ledgerVersion
     * @returns MoveResource
     * @throws ApiError
     */
    public getAccountResource(
        address: Address,
        resourceType: MoveStructTagParam,
        ledgerVersion?: U64,
    ): CancelablePromise<MoveResource> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}/resource/{resource_type}',
            path: {
                'address': address,
                'resource_type': resourceType,
            },
            query: {
                'ledger_version': ledgerVersion,
            },
        });
    }

    /**
     * Get specific account module
     * This endpoint returns the module with a specific name residing at a given
     * account at a specified ledger version (AKA transaction version). If the
     * ledger version is not specified in the request, the latest ledger version
     * is used.
     *
     * The Aptos nodes prune account state history, via a configurable time window (link).
     * If the requested data has been pruned, the server responds with a 404.
     * @param address
     * @param moduleName
     * @param ledgerVersion
     * @returns MoveModuleBytecode
     * @throws ApiError
     */
    public getAccountModule(
        address: Address,
        moduleName: IdentifierWrapper,
        ledgerVersion?: U64,
    ): CancelablePromise<MoveModuleBytecode> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}/module/{module_name}',
            path: {
                'address': address,
                'module_name': moduleName,
            },
            query: {
                'ledger_version': ledgerVersion,
            },
        });
    }

}

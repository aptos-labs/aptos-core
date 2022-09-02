/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Block } from '../models/Block';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class BlocksService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get blocks by height
     * This endpoint allows you to get the transactions in a block
     * and the corresponding block information.
     *
     * If the block is pruned, it will return a 410
     * @param blockHeight
     * @param withTransactions
     * @returns Block
     * @throws ApiError
     */
    public getBlockByHeight(
        blockHeight: number,
        withTransactions?: boolean,
    ): CancelablePromise<Block> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/blocks/by_height/{block_height}',
            path: {
                'block_height': blockHeight,
            },
            query: {
                'with_transactions': withTransactions,
            },
        });
    }

    /**
     * Get blocks by version
     * This endpoint allows you to get the transactions in a block
     * and the corresponding block information given a version in the block.
     *
     * If the block is pruned, it will return a 410
     * @param version
     * @param withTransactions
     * @returns Block
     * @throws ApiError
     */
    public getBlockByVersion(
        version: number,
        withTransactions?: boolean,
    ): CancelablePromise<Block> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/blocks/by_version/{version}',
            path: {
                'version': version,
            },
            query: {
                'with_transactions': withTransactions,
            },
        });
    }

}

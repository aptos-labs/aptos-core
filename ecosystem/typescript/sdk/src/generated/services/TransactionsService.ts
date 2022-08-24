/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Address } from '../models/Address';
import type { EncodeSubmissionRequest } from '../models/EncodeSubmissionRequest';
import type { GasEstimation } from '../models/GasEstimation';
import type { HashValue } from '../models/HashValue';
import type { HexEncodedBytes } from '../models/HexEncodedBytes';
import type { PendingTransaction } from '../models/PendingTransaction';
import type { SubmitTransactionRequest } from '../models/SubmitTransactionRequest';
import type { Transaction } from '../models/Transaction';
import type { TransactionsBatchSubmissionResult } from '../models/TransactionsBatchSubmissionResult';
import type { U64 } from '../models/U64';
import type { UserTransaction } from '../models/UserTransaction';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class TransactionsService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get transactions
     * Get on-chain (meaning, committed) transactions. You may specify from
     * when you want the transactions and how to include in the response.
     * @param start
     * @param limit
     * @returns Transaction
     * @throws ApiError
     */
    public getTransactions(
        start?: U64,
        limit?: number,
    ): CancelablePromise<Array<Transaction>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/transactions',
            query: {
                'start': start,
                'limit': limit,
            },
        });
    }

    /**
     * Submit transaction
     * This endpoint accepts transaction submissions in two formats.
     *
     * To submit a transaction as JSON, you must submit a SubmitTransactionRequest.
     * To build this request, do the following:
     *
     * 1. Encode the transaction as BCS. If you are using a language that has
     * native BCS support, make sure of that library. If not, you may take
     * advantage of /transactions/encode_submission. When using this
     * endpoint, make sure you trust the node you're talking to, as it is
     * possible they could manipulate your request.
     * 2. Sign the encoded transaction and use it to create a TransactionSignature.
     * 3. Submit the request. Make sure to use the "application/json" Content-Type.
     *
     * To submit a transaction as BCS, you must submit a SignedTransaction
     * encoded as BCS. See SignedTransaction in types/src/transaction/mod.rs.
     * @param requestBody
     * @returns PendingTransaction
     * @throws ApiError
     */
    public submitTransaction(
        requestBody: SubmitTransactionRequest,
    ): CancelablePromise<PendingTransaction> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/transactions',
            body: requestBody,
            mediaType: 'application/json',
        });
    }

    /**
     * Get transaction by hash
     * Look up a transaction by its hash. This is the same hash that is returned
     * by the API when submitting a transaction (see PendingTransaction).
     *
     * When given a transaction hash, the server first looks for the transaction
     * in storage (on-chain, committed). If no on-chain transaction is found, it
     * looks the transaction up by hash in the mempool (pending, not yet committed).
     *
     * To create a transaction hash by yourself, do the following:
     * 1. Hash message bytes: "RawTransaction" bytes + BCS bytes of [Transaction](https://aptos-labs.github.io/aptos-core/aptos_types/transaction/enum.Transaction.html).
     * 2. Apply hash algorithm `SHA3-256` to the hash message bytes.
     * 3. Hex-encode the hash bytes with `0x` prefix.
     * @param txnHash
     * @returns Transaction
     * @throws ApiError
     */
    public getTransactionByHash(
        txnHash: HashValue,
    ): CancelablePromise<Transaction> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/transactions/by_hash/{txn_hash}',
            path: {
                'txn_hash': txnHash,
            },
        });
    }

    /**
     * Get transaction by version
     * todo
     * @param txnVersion
     * @returns Transaction
     * @throws ApiError
     */
    public getTransactionByVersion(
        txnVersion: U64,
    ): CancelablePromise<Transaction> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/transactions/by_version/{txn_version}',
            path: {
                'txn_version': txnVersion,
            },
        });
    }

    /**
     * Get account transactions
     * todo
     * @param address
     * @param start
     * @param limit
     * @returns Transaction
     * @throws ApiError
     */
    public getAccountTransactions(
        address: Address,
        start?: U64,
        limit?: number,
    ): CancelablePromise<Array<Transaction>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}/transactions',
            path: {
                'address': address,
            },
            query: {
                'start': start,
                'limit': limit,
            },
        });
    }

    /**
     * @param requestBody
     * @returns TransactionsBatchSubmissionResult
     * @throws ApiError
     */
    public submitBatchTransactions(
        requestBody: Array<SubmitTransactionRequest>,
    ): CancelablePromise<TransactionsBatchSubmissionResult> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/transactions/batch',
            body: requestBody,
            mediaType: 'application/json',
        });
    }

    /**
     * Simulate transaction
     * Simulate submitting a transaction. To use this, you must:
     * - Create a SignedTransaction with a zero-padded signature.
     * - Submit a SubmitTransactionRequest containing a UserTransactionRequest containing that signature.
     *
     * To use this endpoint with BCS, you must submit a SignedTransaction
     * encoded as BCS. See SignedTransaction in types/src/transaction/mod.rs.
     * @param requestBody
     * @returns UserTransaction
     * @throws ApiError
     */
    public simulateTransaction(
        requestBody: SubmitTransactionRequest,
    ): CancelablePromise<Array<UserTransaction>> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/transactions/simulate',
            body: requestBody,
            mediaType: 'application/json',
        });
    }

    /**
     * Encode submission
     * This endpoint accepts an EncodeSubmissionRequest, which internally is a
     * UserTransactionRequestInner (and optionally secondary signers) encoded
     * as JSON, validates the request format, and then returns that request
     * encoded in BCS. The client can then use this to create a transaction
     * signature to be used in a SubmitTransactionRequest, which it then
     * passes to the /transactions POST endpoint.
     *
     * To be clear, this endpoint makes it possible to submit transaction
     * requests to the API from languages that do not have library support for
     * BCS. If you are using an SDK that has BCS support, such as the official
     * Rust, TypeScript, or Python SDKs, you do not need to use this endpoint.
     *
     * To sign a message using the response from this endpoint:
     * - Decode the hex encoded string in the response to bytes.
     * - Sign the bytes to create the signature.
     * - Use that as the signature field in something like Ed25519Signature, which you then use to build a TransactionSignature.
     * @param requestBody
     * @returns HexEncodedBytes
     * @throws ApiError
     */
    public encodeSubmission(
        requestBody: EncodeSubmissionRequest,
    ): CancelablePromise<HexEncodedBytes> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/transactions/encode_submission',
            body: requestBody,
            mediaType: 'application/json',
        });
    }

    /**
     * Estimate gas price
     * @returns GasEstimation
     * @throws ApiError
     */
    public estimateGasPrice(): CancelablePromise<GasEstimation> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/estimate_gas_price',
        });
    }

}

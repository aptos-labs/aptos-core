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
     * Retrieve on-chain committed transactions. The page size and start can be provided to
     * get a specific sequence of transactions.
     *
     * If the version has been pruned, then a 410 will be returned
     * @param start Ledger version to start list of transactions
     *
     * If not provided, defaults to showing the latest transactions
     * @param limit Max number of transactions to retrieve.
     *
     * If not provided, defaults to default page size
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
     * Make sure to use the `application/x.aptos.signed_transaction+bcs` Content-Type.
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
     * @param txnHash Hash of transaction to retrieve
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
     * Retrieves a transaction by a given version.  If the version has been pruned, a 410 will
     * be returned.
     * @param txnVersion Version of transaction to retrieve
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
     * Retrieves transactions from an account.  If the start version is too far in the past
     * a 410 will be returned.
     *
     * If no start version is given, it will start at 0
     * @param address Address of account with or without a `0x` prefix
     * @param start Ledger version to start list of transactions
     *
     * If not provided, defaults to showing the latest transactions
     * @param limit Max number of transactions to retrieve.
     *
     * If not provided, defaults to default page size
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
     * Submit batch transactions
     * This allows you to submit multiple transactions.  The response has three outcomes:
     *
     * 1. All transactions succeed, and it will return a 202
     * 2. Some transactions succeed, and it will return the failed transactions and a 206
     * 3. No transactions succeed, and it will also return the failed transactions and a 206
     *
     * To submit a transaction as JSON, you must submit a SubmitTransactionRequest.
     * To build this request, do the following:
     *
     * 1. Encode the transaction as BCS. If you are using a language that has
     * native BCS support, make sure to use that library. If not, you may take
     * advantage of /transactions/encode_submission. When using this
     * endpoint, make sure you trust the node you're talking to, as it is
     * possible they could manipulate your request.
     * 2. Sign the encoded transaction and use it to create a TransactionSignature.
     * 3. Submit the request. Make sure to use the "application/json" Content-Type.
     *
     * To submit a transaction as BCS, you must submit a SignedTransaction
     * encoded as BCS. See SignedTransaction in types/src/transaction/mod.rs.
     * Make sure to use the `application/x.aptos.signed_transaction+bcs` Content-Type.
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
     * The output of the transaction will have the exact transaction outputs and events that running
     * an actual signed transaction would have.  However, it will not have the associated state
     * hashes, as they are not updated in storage.  This can be used to estimate the maximum gas
     * units for a submitted transaction.
     *
     * To use this, you must:
     * - Create a SignedTransaction with a zero-padded signature.
     * - Submit a SubmitTransactionRequest containing a UserTransactionRequest containing that signature.
     *
     * To use this endpoint with BCS, you must submit a SignedTransaction
     * encoded as BCS. See SignedTransaction in types/src/transaction/mod.rs.
     * @param requestBody
     * @param estimateMaxGasAmount If set to true, the max gas value in the transaction will be ignored
     * and the maximum possible gas will be used
     * @param estimateGasUnitPrice If set to true, the gas unit price in the transaction will be ignored
     * and the estimated value will be used
     * @returns UserTransaction
     * @throws ApiError
     */
    public simulateTransaction(
        requestBody: SubmitTransactionRequest,
        estimateMaxGasAmount?: boolean,
        estimateGasUnitPrice?: boolean,
    ): CancelablePromise<Array<UserTransaction>> {
        return this.httpRequest.request({
            method: 'POST',
            url: '/transactions/simulate',
            query: {
                'estimate_max_gas_amount': estimateMaxGasAmount,
                'estimate_gas_unit_price': estimateGasUnitPrice,
            },
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
     * Currently, the gas estimation is handled by taking the median of the last 100,000 transactions
     * If a user wants to prioritize their transaction and is willing to pay, they can pay more
     * than the gas price.  If they're willing to wait longer, they can pay less.  Note that the
     * gas price moves with the fee market, and should only increase when demand outweighs supply.
     *
     * If there have been no transactions in the last 100,000 transactions, the price will be 1.
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

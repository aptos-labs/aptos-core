/* eslint-disable */
/* tslint:disable */
/*
 * ---------------------------------------------------------------
 * ## THIS FILE WAS GENERATED VIA SWAGGER-TYPESCRIPT-API        ##
 * ##                                                           ##
 * ## AUTHOR: acacode                                           ##
 * ## SOURCE: https://github.com/acacode/swagger-typescript-api ##
 * ---------------------------------------------------------------
 */

import {
  AptosError,
  HexEncodedBytes,
  OnChainTransaction,
  PendingTransaction,
  SubmitTransactionRequest,
  Transaction,
  UserTransactionRequest,
} from "./data-contracts";
import { ContentType, HttpClient, RequestParams } from "./http-client";

export class Transactions<SecurityDataType = unknown> {
  http: HttpClient<SecurityDataType>;

  constructor(http: HttpClient<SecurityDataType>) {
    this.http = http;
  }

  /**
   * No description
   *
   * @tags transactions
   * @name GetTransactions
   * @summary Get transactions
   * @request GET:/transactions
   */
  getTransactions = (query?: { start?: number; limit?: number }, params: RequestParams = {}) =>
    this.http.request<OnChainTransaction[], AptosError>({
      path: `/transactions`,
      method: "GET",
      query: query,
      format: "json",
      ...params,
    });
  /**
   * @description **Submit transaction using JSON without additional tools** * Send [POST /transactions/signing_message](#operation/create-signing-message) to create transaction signing message. * Sign the transaction signing message and create transaction signature. * Submit the user transaction request with the transaction siganture. The request header "Content-Type" must set to "application/json".
   *
   * @tags transactions
   * @name SubmitTransaction
   * @summary Submit transaction
   * @request POST:/transactions
   */
  submitTransaction = (data: SubmitTransactionRequest, params: RequestParams = {}) =>
    this.http.request<PendingTransaction, AptosError>({
      path: `/transactions`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
  /**
   * @description There are two types of transaction identifiers: 1. Transaction hash: included in any transaction JSON respond from server. 2. Transaction version: included in on-chain transaction JSON respond from server. When given transaction hash, server first looks up on-chain transaction by hash; if no on-chain transaction found, then look up transaction by hash in the mempool (pending) transactions. When given a transaction version, server looks up the transaction on-chain by version. To create a transaction hash: 1. Create hash message bytes: "Aptos::Transaction" bytes + BCS bytes of [Transaction](https://aptos-labs.github.io/aptos-core/aptos_types/transaction/enum.Transaction.html). 2. Apply hash algorithm `SHA3-256` to the hash message bytes. 3. Hex-encode the hash bytes with `0x` prefix.
   *
   * @tags transactions
   * @name GetTransaction
   * @summary Get transaction
   * @request GET:/transactions/{txn_hash_or_version}
   */
  getTransaction = (txnHashOrVersion: string, params: RequestParams = {}) =>
    this.http.request<Transaction, AptosError>({
      path: `/transactions/${txnHashOrVersion}`,
      method: "GET",
      format: "json",
      ...params,
    });
  /**
   * @description This API creates transaction signing message for client to create transaction signature. The success response contains hex-encoded signing message bytes. **To sign the message** 1. Client first needs to HEX decode the `message` into bytes. 2. Then sign the bytes to create signature.
   *
   * @tags transactions
   * @name CreateSigningMessage
   * @summary Create transaction signing message
   * @request POST:/transactions/signing_message
   */
  createSigningMessage = (data: UserTransactionRequest, params: RequestParams = {}) =>
    this.http.request<{ message: HexEncodedBytes }, AptosError>({
      path: `/transactions/signing_message`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
}

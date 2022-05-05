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

import { AptosError, TableItemRequest } from "./data-contracts";
import { ContentType, HttpClient, RequestParams } from "./http-client";

export class Tables<SecurityDataType = unknown> {
  http: HttpClient<SecurityDataType>;

  constructor(http: HttpClient<SecurityDataType>) {
    this.http = http;
  }

  /**
   * @description Gets a table item for a table identified by the handle and the key for the item. Key and value types need to be passed in to help with key serialization and value deserialization.
   *
   * @tags state, table
   * @name GetTableItem
   * @summary Get table item by handle and key.
   * @request POST:/tables/{table_handle}/item
   */
  getTableItem = (tableHandle: string, data: TableItemRequest, params: RequestParams = {}) =>
    this.http.request<object, AptosError>({
      path: `/tables/${tableHandle}/item`,
      method: "POST",
      body: data,
      type: ContentType.Json,
      format: "json",
      ...params,
    });
}

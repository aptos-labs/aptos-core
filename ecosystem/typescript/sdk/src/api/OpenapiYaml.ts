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

import { HttpClient, RequestParams } from "./http-client";

export class OpenapiYaml<SecurityDataType = unknown> {
  http: HttpClient<SecurityDataType>;

  constructor(http: HttpClient<SecurityDataType>) {
    this.http = http;
  }

  /**
   * No description
   *
   * @tags general
   * @name GetSpecYaml
   * @summary OpenAPI specification
   * @request GET:/openapi.yaml
   */
  getSpecYaml = (params: RequestParams = {}) =>
    this.http.request<void, void>({
      path: `/openapi.yaml`,
      method: "GET",
      ...params,
    });
}

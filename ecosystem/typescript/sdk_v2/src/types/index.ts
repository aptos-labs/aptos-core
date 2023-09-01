import { Network } from "../utils/api-endpoints";
import { MoveFunction, MoveStruct } from "./generated";

export { Transaction } from "./generated";

export type AnyNumber = number | bigint;
export type HexInput = string | Uint8Array;

export type AptosSettings = {
  readonly network: Network;

  readonly fullnode?: string;

  readonly faucet?: string;

  readonly indexer?: string;

  readonly clientConfig?: ClientConfig;
};

/**
 *
 * Controls the number of results that are returned and the starting position of those results.
 * @param start parameter specifies the starting position of the query result within the set of data. Default is 0.
 * @param limit specifies the maximum number of items or records to return in a query result. Default is 25.
 */
export interface PaginationArgs {
  start?: AnyNumber;
  limit?: number;
}

/**
 * A configuration object we can pass with the request to the server.
 *
 * @param TOKEN - an auth token to send with the request
 * @param HEADERS - extra headers we want to send with the request
 * @param WITH_CREDENTIALS - whether to carry cookies. By default, it is set to true and cookies will be sent
 */
export type ClientConfig = {
  TOKEN?: string;
  HEADERS?: Record<string, string | number | boolean>;
  WITH_CREDENTIALS?: boolean;
};

/**
 * The API request type
 *
 * @param url - the url to make the request to, i.e https://fullnode.aptoslabs.devnet.com/v1
 * @param method - the request method "GET" | "POST"
 * @param endpoint (optional) - the endpoint to make the request to, i.e transactions
 * @param body (optional) - the body of the request
 * @param contentType (optional) - the content type to set the `content-type` header to,
 * by default is set to `application/json`
 * @param params (optional) - query params to add to the request
 * @param originMethod (optional) - the local method the request came from
 * @param overrides (optional) - a `ClientConfig` object type to override request data
 */
export type AptosRequest = {
  url: string;
  method: "GET" | "POST";
  endpoint?: string;
  body?: any;
  contentType?: string;
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  originMethod?: string;
  overrides?: ClientConfig;
};

export type MoveResource = {
  type: MoveResourceType;
  data: {};
};

export type MoveResourceType = `${string}::${string}::${string}`;
export type MoveModuleId = `${string}::${string}`;

export type AccountData = {
  sequence_number: string;
  authentication_key: string;
};

export type MoveModuleBytecode = {
  bytecode: string;
  abi?: MoveModule;
};

export type MoveModule = {
  address: string;
  name: string;
  /**
   * Friends of the module
   */
  friends: Array<MoveModuleId>;
  /**
   * Public functions of the module
   */
  exposed_functions: Array<MoveFunction>;
  /**
   * Structs of the module
   */
  structs: Array<MoveStruct>;
};

export type LedgerVersion = {
  ledgerVersion?: AnyNumber;
};

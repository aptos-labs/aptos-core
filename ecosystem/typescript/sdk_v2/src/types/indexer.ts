/**
 * GENERATED QUERY TYPES FROM GRAPHQL SCHEMA
 *
 * generated types we generate from graphql schema that match the structure of the
 * response type when querying from Hasura schema.
 *
 * These types are used as the return type when making the actual request (usually
 * under the /internal/ folder)
 */

import {
  GetAccountTokensCountQuery,
  GetAccountTransactionsCountQuery,
  GetAccountCoinsDataQuery,
  GetAccountCoinsCountQuery,
  GetAccountOwnedObjectsQuery,
  GetAccountOwnedTokensQuery,
  GetAccountOwnedTokensFromCollectionQuery,
  GetAccountCollectionsWithOwnedTokensQuery,
} from "./generated/operations";

/**
 * CUSTOM RESPONSE TYPES FOR THE END USER
 *
 * To provide a good dev exp, we build custom types derived from the
 * query types to be the response type the end developer/user will
 * work with.
 *
 * These types are used as the return type when calling an sdk api function
 * that calls the function that queries the server (usually under the /api/ folder)
 */
export type GetAccountTokensCountQueryResponse =
  GetAccountTokensCountQuery["current_token_ownerships_v2_aggregate"]["aggregate"];
export type GetAccountTransactionsCountResponse =
  GetAccountTransactionsCountQuery["account_transactions_aggregate"]["aggregate"];
export type GetAccountCoinsCountResponse =
  GetAccountCoinsCountQuery["current_fungible_asset_balances_aggregate"]["aggregate"];
export type GetAccountOwnedObjectsResponse = GetAccountOwnedObjectsQuery["current_objects"];

export type GetAccountOwnedTokensQueryResponse = GetAccountOwnedTokensQuery["current_token_ownerships_v2"];

export type GetAccountOwnedTokensFromCollectionResponse =
  GetAccountOwnedTokensFromCollectionQuery["current_token_ownerships_v2"];
export type GetAccountCollectionsWithOwnedTokenResponse =
  GetAccountCollectionsWithOwnedTokensQuery["current_collection_ownership_v2_view"];
export type GetAccountCoinsDataResponse = GetAccountCoinsDataQuery["current_fungible_asset_balances"];

/**
 * A generic type that being passed by each function and holds an
 * array of properties we can sort the query by
 */
export type OrderBy<T> = Array<{ [K in keyof T]?: OrderByValue }>;
export type OrderByValue =
  | "asc"
  | "asc_nulls_first"
  | "asc_nulls_last"
  | "desc"
  | "desc_nulls_first"
  | "desc_nulls_last";

/**
 * Refers to the token standard we want to query for
 */
export type TokenStandard = "v1" | "v2";

/**
 *
 * Controls the number of results that are returned and the starting position of those results.
 * @param offset parameter specifies the starting position of the query result within the set of data. Default is 0.
 * @param limit specifies the maximum number of items or records to return in a query result. Default is 10.
 */
export interface IndexerPaginationArgs {
  offset?: number | bigint;
  limit?: number;
}

/**
 * The graphql query type to pass into the `queryIndexer` function
 */
export type GraphqlQuery = {
  query: string;
  variables?: {};
};

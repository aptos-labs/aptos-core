import { AnyNumber } from "../bcs/types";

export * from "./indexer";
export * from "./aptos_client";
export * from "./provider";

/**
 * Controls the number of results that are returned and the starting position of those results.
 * limit specifies the maximum number of items or records to return in a query result.
 * offset parameter specifies the starting position of the query result within the set of data.
 * For example, if you want to retrieve records 11-20,
 * you would set the offset parameter to 10 (i.e., the index of the first record to retrieve is 10)
 * and the limit parameter to 10 (i.e., the number of records to retrieve is 10))
 */
export interface PaginationArgs {
    offset?: AnyNumber;
    limit?: number;
}

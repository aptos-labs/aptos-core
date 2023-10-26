import { AnyNumber } from "../bcs/types";
import { HexString, MaybeHexString } from "../utils";
import {
  GetAccountTokensCountQuery,
  GetAccountCoinsDataQuery,
  GetAccountCurrentTokensQuery,
  GetAccountTransactionsCountQuery,
  GetAccountTransactionsDataQuery,
  GetNumberOfDelegatorsQuery,
  GetDelegatedStakingActivitiesQuery,
  GetIndexerLedgerInfoQuery,
  GetTokenActivitiesCountQuery,
  GetTokenActivitiesQuery,
  GetTokenDataQuery,
  GetTokenOwnersDataQuery,
  GetTopUserTransactionsQuery,
  GetUserTransactionsQuery,
  GetOwnedTokensQuery,
  GetTokenOwnedFromCollectionQuery,
  GetCollectionDataQuery,
  GetCollectionsWithOwnedTokensQuery,
  GetTokenCurrentOwnerDataQuery,
  GetOwnedTokensByTokenDataQuery,
  GetAccountCoinsDataCountQuery,
  GetCurrentObjectsQuery,
} from "../indexer/generated/operations";
import {
  GetAccountTokensCount,
  GetAccountCoinsData,
  GetAccountCurrentTokens,
  GetAccountTransactionsCount,
  GetAccountTransactionsData,
  GetNumberOfDelegators,
  GetDelegatedStakingActivities,
  GetIndexerLedgerInfo,
  GetTokenActivities,
  GetTokenActivitiesCount,
  GetTokenData,
  GetTokenOwnersData,
  GetTopUserTransactions,
  GetUserTransactions,
  GetOwnedTokens,
  GetTokenOwnedFromCollection,
  GetCollectionData,
  GetCollectionsWithOwnedTokens,
  GetTokenCurrentOwnerData,
  GetOwnedTokensByTokenData,
  GetAccountCoinsDataCount,
  GetCurrentObjects,
} from "../indexer/generated/queries";
import { ClientConfig, post } from "../client";
import { ApiError } from "./aptos_client";
import {
  Account_Transactions_Order_By,
  Current_Collections_V2_Order_By,
  Current_Collection_Ownership_V2_View_Order_By,
  Current_Fungible_Asset_Balances_Order_By,
  Current_Token_Datas_V2_Order_By,
  Current_Token_Ownerships_V2_Order_By,
  InputMaybe,
  Token_Activities_V2_Order_By,
  User_Transactions_Order_By,
  Current_Objects_Order_By,
} from "../indexer/generated/types";

/**
 * Controls the number of results that are returned and the starting position of those results.
 * limit specifies the maximum number of items or records to return in a query result.
 * offset parameter specifies the starting position of the query result within the set of data.
 * For example, if you want to retrieve records 11-20,
 * you would set the offset parameter to 10 (i.e., the index of the first record to retrieve is 10)
 * and the limit parameter to 10 (i.e., the number of records to retrieve is 10))
 */
export interface IndexerPaginationArgs {
  offset?: AnyNumber;
  limit?: number;
}

/**
 * Holds a generic type that being passed by each function and holds an
 * array of properties we can sort the query by
 */
export type IndexerSortBy<T> = IndexerSortingOptions<T>;

export type IndexerSortingOptions<T> = {
  [K in keyof T]?: T[K] extends InputMaybe<infer U>
    ? IndexerSortingOptions<U> | U | IndexerOrderBy
    : T[K] | IndexerOrderBy;
};

export type IndexerOrderBy = "asc" | "desc";

/**
 * Refers to the token standard we want to query for
 */
export type TokenStandard = "v1" | "v2";

/**
 * The graphql query type to pass into the `queryIndexer` function
 */
export type GraphqlQuery = {
  query: string;
  variables?: {};
};

/**
 * Provides methods for retrieving data from Aptos Indexer.
 * For more detailed Queries specification see
 * {@link https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql}
 *
 * Some methods support optional extra arguments, such as - TokenStandard, IndexerSortBy, IndexerPaginationArgs
 *
 * @param TokenStandard is of type `v1` or `v2` and it refers to the token standard we want to query for.
 * @example An example of how to pass a specific token standard
 * ```
 * {
 *    tokenStandard:"v2"
 * }
 * ```
 *
 * @param IndexerSortBy has a generic type that being passed by each function and holds an
 * array of properties we can sort the query by
 * @example An example of how to sort by a specific field
 * ```
 * {
 *  orderBy: [{ token_standard: "desc" }]
 * }
 * ```
 *
 * @param IndexerPaginationArgs Controls the number of results that are returned and the starting position
 * of those results.
 * limit specifies the maximum number of items or records to return in a query result.
 * offset parameter specifies the starting position of the query result within the set of data.
 * For example, if you want to retrieve records 11-20,
 * you would set the offset parameter to 10 (i.e., the index of the first record to retrieve is 10)
 * and the limit parameter to 10 (i.e., the number of records to retrieve is 10))
 *
 * @example An example of how to set the `limit` and `offset`
 * ```
 * {
 *  { offset: 2, limit: 4 }
 * }
 * ```
 */
export class IndexerClient {
  readonly endpoint: string;

  readonly config: ClientConfig | undefined;

  /**
   * @param endpoint URL of the Aptos Indexer API endpoint.
   */
  constructor(endpoint: string, config?: ClientConfig) {
    this.endpoint = endpoint;
    this.config = config;
  }

  /**
   * Indexer only accepts address in the long format, i.e a 66 chars long -> 0x<64 chars>
   * This method makes sure address is 66 chars long.
   * @param address
   */
  static validateAddress(address: string): void {
    if (address.length < 66) {
      throw new Error(`${address} is less than 66 chars long.`);
    }
  }

  /**
   * Makes axios client call to fetch data from Aptos Indexer.
   *
   * @param graphqlQuery A GraphQL query to pass in the `data` axios call.
   */
  async queryIndexer<T>(graphqlQuery: GraphqlQuery): Promise<T> {
    const response = await post<GraphqlQuery, any>({
      url: this.endpoint,
      body: graphqlQuery,
      overrides: { WITH_CREDENTIALS: false, ...this.config },
    });
    if (response.data.errors) {
      throw new ApiError(
        response.data.errors[0].extensions.code,
        JSON.stringify({
          message: response.data.errors[0].message,
          error_code: response.data.errors[0].extensions.code,
        }),
      );
    }
    return response.data.data;
  }

  /**
   * Queries Indexer Ledger Info
   *
   * @returns GetLedgerInfoQuery response type
   */
  async getIndexerLedgerInfo(): Promise<GetIndexerLedgerInfoQuery> {
    const graphqlQuery = {
      query: GetIndexerLedgerInfo,
    };
    return this.queryIndexer(graphqlQuery);
  }

  // TOKENS //

  /**
   * @deprecated please use `getOwnedTokens` query
   *
   * Queries an Aptos account's NFTs by owner address
   *
   * @param ownerAddress Hex-encoded 32 byte Aptos account address
   * @returns GetAccountCurrentTokensQuery response type
   */
  async getAccountNFTs(
    ownerAddress: MaybeHexString,
    options?: IndexerPaginationArgs,
  ): Promise<GetAccountCurrentTokensQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetAccountCurrentTokens,
      variables: { address, offset: options?.offset, limit: options?.limit },
    };

    return this.queryIndexer<GetAccountCurrentTokensQuery>(graphqlQuery);
  }

  /**
   * Queries a token activities by token address (v2) or token data id (v1)
   *
   * @param idHash token address (v2) or token data id (v1)
   * @returns GetTokenActivitiesQuery response type
   */
  async getTokenActivities(
    token: MaybeHexString,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Token_Activities_V2_Order_By>[];
    },
  ): Promise<GetTokenActivitiesQuery> {
    const tokenAddress = HexString.ensure(token).hex();
    IndexerClient.validateAddress(tokenAddress);

    const whereCondition: any = {
      token_data_id: { _eq: tokenAddress },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }
    const graphqlQuery = {
      query: GetTokenActivities,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };

    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Gets the count of token's activities by token address (v2) or token data id (v1)
   *
   * @param token token address (v2) or token data id (v1)
   * @returns GetTokenActivitiesCountQuery response type
   */
  async getTokenActivitiesCount(token: string): Promise<GetTokenActivitiesCountQuery> {
    const graphqlQuery = {
      query: GetTokenActivitiesCount,
      variables: { token_id: token },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Gets the count of tokens owned by an account
   *
   * @param ownerAddress Owner address
   * @returns AccountTokensCountQuery response type
   */
  async getAccountTokensCount(
    ownerAddress: MaybeHexString,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
    },
  ): Promise<GetAccountTokensCountQuery> {
    const whereCondition: any = {
      owner_address: { _eq: ownerAddress },
      amount: { _gt: "0" },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetAccountTokensCount,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries token data by token address (v2) or token data id (v1)
   *
   * @param token token address (v2) or token data id (v1)
   * @returns GetTokenDataQuery response type
   */
  // :!:>getTokenData
  async getTokenData(
    token: string,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Token_Datas_V2_Order_By>[];
    },
  ): Promise<GetTokenDataQuery> {
    const tokenAddress = HexString.ensure(token).hex();
    IndexerClient.validateAddress(tokenAddress);

    const whereCondition: any = {
      token_data_id: { _eq: tokenAddress },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }
    const graphqlQuery = {
      query: GetTokenData,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  } // <:!:getTokenData

  /**
   * Queries token owners data by token address (v2) or token data id (v1).
   * This query returns historical owners data.
   *
   * To fetch token v2 standard, pass in the optional `tokenStandard` parameter and
   * dont pass `propertyVersion` parameter (as propertyVersion only compatible with v1 standard)
   *
   * @param token token address (v2) or token data id (v1)
   * @param propertyVersion Property version (optional) - only compatible with token v1 standard
   * @returns GetTokenOwnersDataQuery response type
   */
  async getTokenOwnersData(
    token: string,
    propertyVersion?: number,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Token_Ownerships_V2_Order_By>[];
    },
  ): Promise<GetTokenOwnersDataQuery> {
    const tokenAddress = HexString.ensure(token).hex();
    IndexerClient.validateAddress(tokenAddress);

    const whereCondition: any = {
      token_data_id: { _eq: tokenAddress },
      amount: { _gt: "0" },
    };

    if (propertyVersion) {
      whereCondition.property_version_v1 = { _eq: propertyVersion };
    }

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const graphqlQuery = {
      query: GetTokenOwnersData,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries current token owner data by token address (v2) or token data id (v1).
   * This query returns the current token owner data.
   *
   * To fetch token v2 standard, pass in the optional `tokenStandard` parameter and
   * dont pass `propertyVersion` parameter (as propertyVersion only compatible with v1 standard)
   *
   * @param token token address (v2) or token data id (v1)
   * @param propertyVersion Property version (optional) - only compatible with token v1 standard
   * @returns GetTokenCurrentOwnerDataQuery response type
   */
  async getTokenCurrentOwnerData(
    token: string,
    propertyVersion?: number,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Token_Ownerships_V2_Order_By>[];
    },
  ): Promise<GetTokenCurrentOwnerDataQuery> {
    const tokenAddress = HexString.ensure(token).hex();
    IndexerClient.validateAddress(tokenAddress);

    const whereCondition: any = {
      token_data_id: { _eq: tokenAddress },
      amount: { _gt: "0" },
    };

    if (propertyVersion) {
      whereCondition.property_version_v1 = { _eq: propertyVersion };
    }

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const graphqlQuery = {
      query: GetTokenCurrentOwnerData,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries account's current owned tokens.
   * This query returns all tokens (v1 and v2 standards) an account owns, including NFTs, fungible, soulbound, etc.
   * If you want to get only the token from a specific standrd, you can pass an optional tokenStandard param
   *
   * @param ownerAddress The token owner address we want to get the tokens for
   * @returns GetOwnedTokensQuery response type
   */
  async getOwnedTokens(
    ownerAddress: MaybeHexString,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Token_Ownerships_V2_Order_By>[];
    },
  ): Promise<GetOwnedTokensQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);

    const whereCondition: any = {
      owner_address: { _eq: address },
      amount: { _gt: 0 },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const graphqlQuery = {
      query: GetOwnedTokens,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries account's current owned tokens by token address (v2) or token data id (v1).
   *
   * @param token token address (v2) or token data id (v1)
   * @returns GetOwnedTokensByTokenDataQuery response type
   */
  async getOwnedTokensByTokenData(
    token: MaybeHexString,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Token_Ownerships_V2_Order_By>[];
    },
  ): Promise<GetOwnedTokensByTokenDataQuery> {
    const address = HexString.ensure(token).hex();
    IndexerClient.validateAddress(address);

    const whereCondition: any = {
      token_data_id: { _eq: address },
      amount: { _gt: 0 },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const graphqlQuery = {
      query: GetOwnedTokensByTokenData,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries all tokens of a specific collection that an account owns by the collection address
   *
   * @param ownerAddress owner address that owns the tokens
   * @param collectionAddress the collection address
   * @returns GetTokenOwnedFromCollectionQuery response type
   */
  async getTokenOwnedFromCollectionAddress(
    ownerAddress: MaybeHexString,
    collectionAddress: string,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Token_Ownerships_V2_Order_By>[];
    },
  ): Promise<GetTokenOwnedFromCollectionQuery> {
    const ownerHexAddress = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(ownerHexAddress);

    const collectionHexAddress = HexString.ensure(collectionAddress).hex();
    IndexerClient.validateAddress(collectionHexAddress);

    const whereCondition: any = {
      owner_address: { _eq: ownerHexAddress },
      current_token_data: { collection_id: { _eq: collectionHexAddress } },
      amount: { _gt: 0 },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const graphqlQuery = {
      query: GetTokenOwnedFromCollection,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries all tokens of a specific collection that an account owns by the collection name and collection
   * creator address
   *
   * @param ownerAddress owner address that owns the tokens
   * @param collectionName the collection name
   * @param creatorAddress the collection creator address
   * @returns GetTokenOwnedFromCollectionQuery response type
   */
  async getTokenOwnedFromCollectionNameAndCreatorAddress(
    ownerAddress: MaybeHexString,
    collectionName: string,
    creatorAddress: MaybeHexString,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
    },
  ): Promise<GetTokenOwnedFromCollectionQuery> {
    const collectionAddress = await this.getCollectionAddress(creatorAddress, collectionName, extraArgs);
    const tokens = await this.getTokenOwnedFromCollectionAddress(ownerAddress, collectionAddress, extraArgs);
    return tokens;
  }

  /**
   * Queries data of a specific collection by the collection creator address and the collection name.
   *
   * if, for some reason, a creator account has 2 collections with the same name in v1 and v2,
   * can pass an optional `tokenStandard` parameter to query a specific standard
   *
   * @param creatorAddress the collection creator address
   * @param collectionName the collection name
   * @returns GetCollectionDataQuery response type
   */
  async getCollectionData(
    creatorAddress: MaybeHexString,
    collectionName: string,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Collections_V2_Order_By>[];
    },
  ): Promise<GetCollectionDataQuery> {
    const address = HexString.ensure(creatorAddress).hex();
    IndexerClient.validateAddress(address);

    const whereCondition: any = {
      collection_name: { _eq: collectionName },
      creator_address: { _eq: address },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const graphqlQuery = {
      query: GetCollectionData,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries a collection address.
   *
   * @param creatorAddress the collection creator address
   * @param collectionName the collection name
   * @returns the collection address
   */
  async getCollectionAddress(
    creatorAddress: MaybeHexString,
    collectionName: string,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      orderBy?: IndexerSortBy<Current_Collections_V2_Order_By>[];
    },
  ): Promise<string> {
    return (await this.getCollectionData(creatorAddress, collectionName, extraArgs)).current_collections_v2[0]
      .collection_id;
  }

  /**
   * Queries for all collections that an account has tokens for.
   *
   * @param ownerAddress the account address that owns the tokens
   * @returns GetCollectionsWithOwnedTokensQuery response type
   */
  async getCollectionsWithOwnedTokens(
    ownerAddress: MaybeHexString,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Collection_Ownership_V2_View_Order_By>[];
    },
  ): Promise<GetCollectionsWithOwnedTokensQuery> {
    const ownerHexAddress = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(ownerHexAddress);

    const whereCondition: any = {
      owner_address: { _eq: ownerHexAddress },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.current_collection = { token_standard: { _eq: extraArgs?.tokenStandard } };
    }

    const graphqlQuery = {
      query: GetCollectionsWithOwnedTokens,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  // TRANSACTIONS //

  /**
   * Gets the count of transactions submitted by an account
   *
   * @param address Account address
   * @returns GetAccountTransactionsCountQuery response type
   */
  async getAccountTransactionsCount(accountAddress: MaybeHexString): Promise<GetAccountTransactionsCountQuery> {
    const address = HexString.ensure(accountAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetAccountTransactionsCount,
      variables: { address },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries an account transactions data
   *
   * @param address Account address
   * @returns GetAccountTransactionsDataQuery response type
   */
  async getAccountTransactionsData(
    accountAddress: MaybeHexString,
    extraArgs?: {
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Account_Transactions_Order_By>[];
    },
  ): Promise<GetAccountTransactionsDataQuery> {
    const address = HexString.ensure(accountAddress).hex();
    IndexerClient.validateAddress(address);

    const whereCondition: any = {
      account_address: { _eq: address },
    };

    const graphqlQuery = {
      query: GetAccountTransactionsData,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries top user transactions
   *
   * @param limit
   * @returns GetTopUserTransactionsQuery response type
   */
  async getTopUserTransactions(limit: number): Promise<GetTopUserTransactionsQuery> {
    const graphqlQuery = {
      query: GetTopUserTransactions,
      variables: { limit },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries top user transactions
   *
   * @param startVersion optional - can be set to tell indexer what version to start from
   * @returns GetUserTransactionsQuery response type
   */
  async getUserTransactions(extraArgs?: {
    startVersion?: number;
    options?: IndexerPaginationArgs;
    orderBy?: IndexerSortBy<User_Transactions_Order_By>[];
  }): Promise<GetUserTransactionsQuery> {
    const whereCondition: any = {
      version: { _lte: extraArgs?.startVersion },
    };

    const graphqlQuery = {
      query: GetUserTransactions,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  // STAKING //

  /**
   * Queries delegated staking activities
   *
   * @param delegatorAddress Delegator address
   * @param poolAddress Pool address
   * @returns GetDelegatedStakingActivitiesQuery response type
   */
  async getDelegatedStakingActivities(
    delegatorAddress: MaybeHexString,
    poolAddress: MaybeHexString,
  ): Promise<GetDelegatedStakingActivitiesQuery> {
    const delegator = HexString.ensure(delegatorAddress).hex();
    const pool = HexString.ensure(poolAddress).hex();
    IndexerClient.validateAddress(delegator);
    IndexerClient.validateAddress(pool);
    const graphqlQuery = {
      query: GetDelegatedStakingActivities,
      variables: {
        delegatorAddress: delegator,
        poolAddress: pool,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries current number of delegators in a pool
   *
   * @returns GetNumberOfDelegatorsQuery response type
   */
  async getNumberOfDelegators(poolAddress: MaybeHexString): Promise<GetNumberOfDelegatorsQuery> {
    const address = HexString.ensure(poolAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetNumberOfDelegators,
      variables: { poolAddress: address },
    };
    return this.queryIndexer(graphqlQuery);
  }

  // ACCOUNT //

  /**
   * Queries an account coin data
   *
   * @param ownerAddress Owner address
   * @returns GetAccountCoinsDataQuery response type
   */
  async getAccountCoinsData(
    ownerAddress: MaybeHexString,
    extraArgs?: {
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Fungible_Asset_Balances_Order_By>[];
    },
  ): Promise<GetAccountCoinsDataQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);

    const whereCondition: any = {
      owner_address: { _eq: address },
    };

    const graphqlQuery = {
      query: GetAccountCoinsData,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };

    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries an account coin data count
   *
   * @param ownerAddress Owner address
   * @returns GetAccountCoinsDataCountQuery response type
   */
  async getAccountCoinsDataCount(ownerAddress: MaybeHexString): Promise<GetAccountCoinsDataCountQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);

    const graphqlQuery = {
      query: GetAccountCoinsDataCount,
      variables: {
        address,
      },
    };

    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries an account owned objects
   *
   * @param ownerAddress Owner address
   * @returns GetCurrentObjectsQuery response type
   */
  async getAccountOwnedObjects(
    ownerAddress: MaybeHexString,
    extraArgs?: {
      options?: IndexerPaginationArgs;
      orderBy?: IndexerSortBy<Current_Objects_Order_By>[];
    },
  ): Promise<GetCurrentObjectsQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);

    const whereCondition: any = {
      owner_address: { _eq: address },
    };

    const graphqlQuery = {
      query: GetCurrentObjects,
      variables: {
        where_condition: whereCondition,
        offset: extraArgs?.options?.offset,
        limit: extraArgs?.options?.limit,
        order_by: extraArgs?.orderBy,
      },
    };
    return this.queryIndexer(graphqlQuery);
  }
}

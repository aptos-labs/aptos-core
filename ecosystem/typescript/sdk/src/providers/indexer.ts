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
} from "../indexer/generated/queries";
import { ClientConfig, post } from "../client";
import { ApiError } from "./aptos_client";

/**
 * Controls the number of results that are returned and the starting position of those results.
 * limit specifies the maximum number of items or records to return in a query result.
 * offset parameter specifies the starting position of the query result within the set of data.
 * For example, if you want to retrieve records 11-20,
 * you would set the offset parameter to 10 (i.e., the index of the first record to retrieve is 10)
 * and the limit parameter to 10 (i.e., the number of records to retrieve is 10))
 */
interface PaginationArgs {
  offset?: AnyNumber;
  limit?: number;
}

type TokenStandard = "v1" | "v2";

type GraphqlQuery = {
  query: string;
  variables?: {};
};
/**
 * Provides methods for retrieving data from Aptos Indexer.
 * For more detailed Queries specification see
 * {@link https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql}
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

  /**
   * Queries an Aptos account's NFTs by owner address
   *
   * @param ownerAddress Hex-encoded 32 byte Aptos account address
   * @returns GetAccountCurrentTokensQuery response type
   */
  async getAccountNFTs(ownerAddress: MaybeHexString, options?: PaginationArgs): Promise<GetAccountCurrentTokensQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetAccountCurrentTokens,
      variables: { address, offset: options?.offset, limit: options?.limit },
    };

    return this.queryIndexer<GetAccountCurrentTokensQuery>(graphqlQuery);
  }

  /**
   * Queries a token activities by token id hash
   *
   * @param idHash token id hash
   * @returns GetTokenActivitiesQuery response type
   */
  async getTokenActivities(idHash: string, options?: PaginationArgs): Promise<GetTokenActivitiesQuery> {
    const graphqlQuery = {
      query: GetTokenActivities,
      variables: { idHash, offset: options?.offset, limit: options?.limit },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries an account coin data
   *
   * @param ownerAddress Owner address
   * @returns GetAccountCoinsDataQuery response type
   */
  async getAccountCoinsData(ownerAddress: MaybeHexString, options?: PaginationArgs): Promise<GetAccountCoinsDataQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetAccountCoinsData,
      variables: { owner_address: address, offset: options?.offset, limit: options?.limit },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Gets the count of tokens owned by an account
   *
   * @param ownerAddress Owner address
   * @returns AccountTokensCountQuery response type
   */
  async getAccountTokensCount(ownerAddress: MaybeHexString): Promise<GetAccountTokensCountQuery> {
    const address = HexString.ensure(ownerAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetAccountTokensCount,
      variables: { owner_address: address },
    };
    return this.queryIndexer(graphqlQuery);
  }

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
    options?: PaginationArgs,
  ): Promise<GetAccountTransactionsDataQuery> {
    const address = HexString.ensure(accountAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetAccountTransactionsData,
      variables: { address, offset: options?.offset, limit: options?.limit },
    };
    return this.queryIndexer(graphqlQuery);
  }

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
   * Gets the count of token's activities
   *
   * @param tokenId Token ID
   * @returns GetTokenActivitiesCountQuery response type
   */
  async getTokenActivitiesCount(tokenId: string): Promise<GetTokenActivitiesCountQuery> {
    const graphqlQuery = {
      query: GetTokenActivitiesCount,
      variables: { token_id: tokenId },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries token data
   *
   * @param tokenId Token ID address
   * @returns GetTokenDataQuery response type
   */
  async getTokenData(
    tokenId: string,
    extraArgs?: {
      tokenStandard?: TokenStandard;
    },
  ): Promise<GetTokenDataQuery> {
    const tokenAddress = HexString.ensure(tokenId).hex();
    IndexerClient.validateAddress(tokenAddress);

    const whereCondition: any = {
      token_data_id: { _eq: tokenAddress },
    };

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }
    const graphqlQuery = {
      query: GetTokenData,
      variables: { where_condition: whereCondition },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries token owners data. This query returns historical owners data
   * To fetch token v2 standard, pass in the optional `tokenStandard` parameter and
   * dont pass `propertyVersion` parameter (as propertyVersion only compatible with v1 standard)
   *
   * @param tokenId Token ID
   * @param propertyVersion Property version (optional) - only compatible with token v1 standard
   * @returns GetTokenOwnersDataQuery response type
   */
  async getTokenOwnersData(
    tokenId: string,
    propertyVersion?: number,
    extraArgs?: {
      tokenStandard?: TokenStandard;
    },
  ): Promise<GetTokenOwnersDataQuery> {
    const tokenAddress = HexString.ensure(tokenId).hex();
    IndexerClient.validateAddress(tokenAddress);

    const whereCondition: any = {
      token_data_id: { _eq: tokenAddress },
    };

    if (propertyVersion) {
      whereCondition.property_version_v1 = { _eq: propertyVersion };
    }

    if (extraArgs?.tokenStandard) {
      whereCondition.token_standard = { _eq: extraArgs?.tokenStandard };
    }

    const graphqlQuery = {
      query: GetTokenOwnersData,
      variables: { where_condition: whereCondition },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries token current owner data. This query returns the current token owner data.
   * To fetch token v2 standard, pass in the optional `tokenStandard` parameter and
   * dont pass `propertyVersion` parameter (as propertyVersion only compatible with v1 standard)
   *
   * @param tokenId Token ID
   * @param propertyVersion Property version (optional) - only compatible with token v1 standard
   * @returns GetTokenCurrentOwnerDataQuery response type
   */
  async getTokenCurrentOwnerData(
    tokenId: string,
    propertyVersion?: number,
    extraArgs?: {
      tokenStandard?: TokenStandard;
    },
  ): Promise<GetTokenCurrentOwnerDataQuery> {
    const tokenAddress = HexString.ensure(tokenId).hex();
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
      variables: { where_condition: whereCondition },
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
   * @returns GetUserTransactionsQuery response type
   */
  async getUserTransactions(startVersion?: number, options?: PaginationArgs): Promise<GetUserTransactionsQuery> {
    const graphqlQuery = {
      query: GetUserTransactions,
      variables: { start_version: startVersion, offset: options?.offset, limit: options?.limit },
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

  /**
   * Queries account's current owned tokens.
   * This query returns all tokens (v1 and v2 standards) an account owns, including NFTs, fungible, soulbound, etc.
   * If you want to get only the token from a specific standrd, you can pass an optional tokenStandard param
   * @example An example of how to pass a specific token standard
   * ```
   * {
   *    tokenStandard:"v2"
   * }
   * ```
   * @param ownerAddress The token owner address we want to get the tokens for
   * @returns GetOwnedTokensQuery response type
   */
  async getOwnedTokens(
    ownerAddress: MaybeHexString,
    extraArgs?: {
      tokenStandard?: TokenStandard;
      options?: PaginationArgs;
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
      options?: PaginationArgs;
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
      options?: PaginationArgs;
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
      options?: PaginationArgs;
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
      options?: PaginationArgs;
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
      },
    };
    return this.queryIndexer(graphqlQuery);
  }
}

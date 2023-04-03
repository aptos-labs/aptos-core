import axios from "axios";

import { AnyNumber } from "../bcs/types";
import { HexString, MaybeHexString } from "../utils";
import {
  GetAccountTokensCountQuery,
  GetAccountCoinsDataQuery,
  GetAccountCurrentTokensQuery,
  GetAccountTransactionsCountQuery,
  GetAccountTransactionsDataQuery,
  GetCurrentDelegatorBalancesCountQuery,
  GetDelegatedStakingActivitiesQuery,
  GetIndexerLedgerInfoQuery,
  GetTokenActivitiesCountQuery,
  GetTokenActivitiesQuery,
  GetTokenDataQuery,
  GetTokenOwnersDataQuery,
  GetTopUserTransactionsQuery,
  GetUserTransactionsQuery,
} from "../indexer/generated/operations";
import {
  GetAccountTokensCount,
  GetAccountCoinsData,
  GetAccountCurrentTokens,
  GetAccountTransactionsCount,
  GetAccountTransactionsData,
  GetCurrentDelegatorBalancesCount,
  GetDelegatedStakingActivities,
  GetIndexerLedgerInfo,
  GetTokenActivities,
  GetTokenActivitiesCount,
  GetTokenData,
  GetTokenOwnersData,
  GetTopUserTransactions,
  GetUserTransactions,
} from "../indexer/generated/queries";

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
  endpoint: string;

  /**
   * @param endpoint URL of the Aptos Indexer API endpoint.
   */
  constructor(endpoint: string) {
    this.endpoint = endpoint;
  }

  /**
   * Indexer only accepts address in the long format, i.e a 66 chars long -> 0x<64 chars>
   * This method makes sure address is 66 chars long.
   * @param address
   */
  static validateAddress(address: string): void {
    if (address.length < 66) {
      throw new Error("Address needs to be 66 chars long.");
    }
  }

  /**
   * Builds a axios client call to fetch data from Aptos Indexer.
   *
   * @param graphqlQuery A GraphQL query to pass in the `data` axios call.
   */
  async queryIndexer<T>(graphqlQuery: GraphqlQuery): Promise<T> {
    const { data } = await axios.post(this.endpoint, graphqlQuery);
    if (data.errors) {
      throw new Error(`Indexer data error ${JSON.stringify(data.errors, null, " ")}`);
    }
    return data.data;
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
   * @param tokenId Token ID
   * @returns GetTokenDataQuery response type
   */
  async getTokenData(tokenId: string): Promise<GetTokenDataQuery> {
    const graphqlQuery = {
      query: GetTokenData,
      variables: { token_id: tokenId },
    };
    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries token owners data
   *
   * @param tokenId Token ID
   * @param propertyVersion Property version
   * @returns GetTokenOwnersDataQuery response type
   */
  async getTokenOwnersData(tokenId: string, propertyVersion: number): Promise<GetTokenOwnersDataQuery> {
    const graphqlQuery = {
      query: GetTokenOwnersData,
      variables: { token_id: tokenId, property_version: propertyVersion },
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
   * Queries current delegator balances count
   *
   * @returns GetCurrentDelegatorBalancesCountQuery response type
   */
  async getCurrentDelegatorBalancesCount(poolAddress: MaybeHexString): Promise<GetCurrentDelegatorBalancesCountQuery> {
    const address = HexString.ensure(poolAddress).hex();
    IndexerClient.validateAddress(address);
    const graphqlQuery = {
      query: GetCurrentDelegatorBalancesCount,
      variables: { poolAddress: address },
    };
    return this.queryIndexer(graphqlQuery);
  }
}

import { AnyNumber } from "../bcs/types";
import axios from "axios";
import { print } from "graphql";

import { MaybeHexString } from "../hex_string";
import { GetAccountCurrentTokens, GetTokenActivities } from "../indexer/generated/queries";
import { GetAccountCurrentTokensQuery, GetTokenActivitiesQuery } from "../indexer/generated/operations";

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
 * For more detailed Queries specification see {@link https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql}
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
   * Builds a axios client call to fetch data from Aptos Indexer.
   *
   * @param graphqlQuery A GraphQL query to pass in the `data` axios call.
   */
  private async queryIndexer(graphqlQuery: GraphqlQuery): Promise<any> {
    try {
      const { data } = await axios({
        url: this.endpoint,
        method: "post",
        data: graphqlQuery,
      });
      if (data.errors) {
        console.log("data error", data.errors);
        return data.errors;
      }
      return data.data;
    } catch (error) {
      console.log("error", error);
    }
  }

  /**
   * Queries an Aptos account's NFTs by address
   *
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @returns GetAccountCurrentTokensQuery response type
   */
  async getAccountNFTs(ownerAddress: MaybeHexString, options?: PaginationArgs): Promise<GetAccountCurrentTokensQuery> {
    const graphqlQuery = {
      query: print(GetAccountCurrentTokens),
      variables: { address: ownerAddress, offset: options?.offset, limit: options?.limit },
    };

    return this.queryIndexer(graphqlQuery);
  }

  /**
   * Queries a token activities by token id hash
   *
   * @param idHash token id hash
   * @returns GetTokenActivitiesQuery response type
   */
  async getTokenActivities(idHash: string): Promise<GetTokenActivitiesQuery> {
    const graphqlQuery = {
      query: print(GetTokenActivities),
      variables: { idHash },
    };
    return this.queryIndexer(graphqlQuery);
  }
}

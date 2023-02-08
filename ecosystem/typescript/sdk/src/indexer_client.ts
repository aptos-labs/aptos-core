import { AnyNumber } from "./bcs/types";
import axios from "axios";
import { print } from "graphql/language/printer";

import { MaybeHexString } from "./hex_string";
import { GetAccountCurrentTokens, GetTokenActivities } from "./indexer/generated/queries";
import { GetAccountCurrentTokensQuery, GetTokenActivitiesQuery } from "./indexer/generated/operations";

interface PaginationArgs {
  offset?: AnyNumber;
  limit?: number;
}

type GraphqlQuery = {
  query: string;
  variables?: {};
};

export class IndexerClient {
  endpoint: string;

  constructor(url: string) {
    this.endpoint = url;
  }

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

  async getAccountNFTs(ownerAddress: MaybeHexString, options?: PaginationArgs): Promise<GetAccountCurrentTokensQuery> {
    const graphqlQuery = {
      query: print(GetAccountCurrentTokens),
      variables: { address: ownerAddress, offset: options?.offset, limit: options?.limit },
    };

    return this.queryIndexer(graphqlQuery);
  }

  async getTokenActivities(idHash: string): Promise<GetTokenActivitiesQuery> {
    const graphqlQuery = {
      query: print(GetTokenActivities),
      variables: { idHash },
    };
    return this.queryIndexer(graphqlQuery);
  }
}

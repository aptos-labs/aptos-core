import { AnyNumber } from "./bcs/types";
import axios from "axios";

import { MaybeHexString } from "./hex_string";

const headers = {
  Accept: "application/json",
  "Content-Type": "application/json",
};

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

  // TODO use graphql codegen for queries
  async getAccountNFTs(ownerAddress: MaybeHexString, query?: PaginationArgs): Promise<any> {
    const graphqlQuery = {
      query: `query AccountCurrentTokenOwnership($owner_address: String, $limit: Int, $offset: Int) {
        current_token_ownerships(
          where: { owner_address: { _eq: $owner_address }, amount: { _gt: "0" } }
          limit: $limit
          offset: $offset
        ) {
          token_data_id_hash
          name
          collection_name
          table_type
          property_version
          amount
        }
      }
    `,
      variables: { owner_address: ownerAddress, limit: query?.limit, offset: query?.offset },
    };

    return this.queryIndexer(graphqlQuery);
  }
}

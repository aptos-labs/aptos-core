import { ApolloClient, InMemoryCache, gql, NormalizedCacheObject } from "@apollo/client/core";
import { AnyNumber } from "./bcs/types";

import { MaybeHexString } from "./hex_string";

interface PaginationArgs {
  offset?: AnyNumber;
  limit?: number;
}

export class IndexerClient {
  readonly indexer: ApolloClient<NormalizedCacheObject>;

  constructor(url: string) {
    this.indexer = new ApolloClient({
      uri: url,
      cache: new InMemoryCache(),
    });
  }
  // TODO use graphql codegen for queries
  async getAccountNFTs(ownerAddress: MaybeHexString, query: PaginationArgs): Promise<any> {
    const response = await this.indexer.query({
      query: gql`
        query AccountCurrentTokenOwnership($owner_address: String, $limit: Int, $offset: Int) {
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
      variables: { owner_address: ownerAddress, limit: query.limit, offset: query.offset },
    });
    return response.data.current_token_ownerships;
  }
}

import * as Types from './operations';

import { GraphQLClient } from 'graphql-request';
import * as Dom from 'graphql-request/dist/types.dom';
export const TokenDataFieldsFragmentDoc = `
    fragment TokenDataFields on current_token_datas {
  creator_address
  collection_name
  description
  metadata_uri
  name
  token_data_id_hash
  collection_data_id_hash
}
    `;
export const CollectionDataFieldsFragmentDoc = `
    fragment CollectionDataFields on current_collection_datas {
  metadata_uri
  supply
  description
  collection_name
  collection_data_id_hash
  table_handle
  creator_address
}
    `;
export const GetAccountCurrentTokens = `
    query getAccountCurrentTokens($address: String!, $offset: Int, $limit: Int) {
  current_token_ownerships(
    where: {owner_address: {_eq: $address}, amount: {_gt: 0}}
    order_by: [{last_transaction_version: desc}, {creator_address: asc}, {collection_name: asc}, {name: asc}]
    offset: $offset
    limit: $limit
  ) {
    amount
    current_token_data {
      ...TokenDataFields
    }
    current_collection_data {
      ...CollectionDataFields
    }
    last_transaction_version
    property_version
  }
}
    ${TokenDataFieldsFragmentDoc}
${CollectionDataFieldsFragmentDoc}`;
export const GetTokenActivities = `
    query getTokenActivities($idHash: String!) {
  token_activities(where: {token_data_id_hash: {_eq: $idHash}}) {
    creator_address
    collection_name
    name
    token_data_id_hash
    collection_data_id_hash
    from_address
    to_address
    transaction_version
    transaction_timestamp
    property_version
    transfer_type
    event_sequence_number
    token_amount
  }
}
    `;

export type SdkFunctionWrapper = <T>(action: (requestHeaders?:Record<string, string>) => Promise<T>, operationName: string, operationType?: string) => Promise<T>;


const defaultWrapper: SdkFunctionWrapper = (action, _operationName, _operationType) => action();

export function getSdk(client: GraphQLClient, withWrapper: SdkFunctionWrapper = defaultWrapper) {
  return {
    getAccountCurrentTokens(variables: Types.GetAccountCurrentTokensQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCurrentTokensQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountCurrentTokensQuery>(GetAccountCurrentTokens, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountCurrentTokens', 'query');
    },
    getTokenActivities(variables: Types.GetTokenActivitiesQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenActivitiesQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenActivitiesQuery>(GetTokenActivities, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenActivities', 'query');
    }
  };
}
export type Sdk = ReturnType<typeof getSdk>;
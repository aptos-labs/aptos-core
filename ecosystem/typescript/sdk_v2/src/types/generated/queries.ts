import * as Types from './operations';

import { GraphQLClient } from 'graphql-request';
import * as Dom from 'graphql-request/dist/types.dom';
export const CurrentTokenOwnershipFieldsFragmentDoc = `
    fragment CurrentTokenOwnershipFields on current_token_ownerships_v2 {
  token_standard
  token_properties_mutated_v1
  token_data_id
  table_type_v1
  storage_id
  property_version_v1
  owner_address
  last_transaction_version
  last_transaction_timestamp
  is_soulbound_v2
  is_fungible_v2
  amount
  current_token_data {
    collection_id
    description
    is_fungible_v2
    largest_property_version_v1
    last_transaction_timestamp
    last_transaction_version
    maximum
    supply
    token_data_id
    token_name
    token_properties
    token_standard
    token_uri
    current_collection {
      collection_id
      collection_name
      creator_address
      current_supply
      description
      last_transaction_timestamp
      last_transaction_version
      max_supply
      mutable_description
      mutable_uri
      table_handle_v1
      token_standard
      total_minted_v2
      uri
    }
  }
}
    `;
export const GetAccountCoinsCount = `
    query getAccountCoinsCount($address: String) {
  current_fungible_asset_balances_aggregate(
    where: {owner_address: {_eq: $address}}
  ) {
    aggregate {
      count
    }
  }
}
    `;
export const GetAccountCoinsData = `
    query getAccountCoinsData($where_condition: current_fungible_asset_balances_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_fungible_asset_balances_order_by!]) {
  current_fungible_asset_balances(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
    amount
    asset_type
    is_frozen
    is_primary
    last_transaction_timestamp
    last_transaction_version
    owner_address
    storage_id
    token_standard
    metadata {
      token_standard
      symbol
      supply_aggregator_table_key_v1
      supply_aggregator_table_handle_v1
      project_uri
      name
      last_transaction_version
      last_transaction_timestamp
      icon_uri
      decimals
      creator_address
      asset_type
    }
  }
}
    `;
export const GetAccountCollectionsWithOwnedTokens = `
    query getAccountCollectionsWithOwnedTokens($where_condition: current_collection_ownership_v2_view_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_collection_ownership_v2_view_order_by!]) {
  current_collection_ownership_v2_view(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
    current_collection {
      collection_id
      collection_name
      creator_address
      current_supply
      description
      last_transaction_timestamp
      last_transaction_version
      mutable_description
      max_supply
      mutable_uri
      table_handle_v1
      token_standard
      total_minted_v2
      uri
    }
    collection_id
    collection_name
    collection_uri
    creator_address
    distinct_tokens
    last_transaction_version
    owner_address
    single_token_uri
  }
}
    `;
export const GetAccountOwnedObjects = `
    query getAccountOwnedObjects($where_condition: current_objects_bool_exp, $offset: Int, $limit: Int, $order_by: [current_objects_order_by!]) {
  current_objects(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
    allow_ungated_transfer
    state_key_hash
    owner_address
    object_address
    last_transaction_version
    last_guid_creation_num
    is_deleted
  }
}
    `;
export const GetAccountOwnedTokens = `
    query getAccountOwnedTokens($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
  current_token_ownerships_v2(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
    ...CurrentTokenOwnershipFields
  }
}
    ${CurrentTokenOwnershipFieldsFragmentDoc}`;
export const GetAccountOwnedTokensByTokenData = `
    query getAccountOwnedTokensByTokenData($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
  current_token_ownerships_v2(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
    ...CurrentTokenOwnershipFields
  }
}
    ${CurrentTokenOwnershipFieldsFragmentDoc}`;
export const GetAccountOwnedTokensFromCollection = `
    query getAccountOwnedTokensFromCollection($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
  current_token_ownerships_v2(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
    ...CurrentTokenOwnershipFields
  }
}
    ${CurrentTokenOwnershipFieldsFragmentDoc}`;
export const GetAccountTokensCount = `
    query getAccountTokensCount($where_condition: current_token_ownerships_v2_bool_exp, $offset: Int, $limit: Int) {
  current_token_ownerships_v2_aggregate(
    where: $where_condition
    offset: $offset
    limit: $limit
  ) {
    aggregate {
      count
    }
  }
}
    `;
export const GetAccountTransactionsCount = `
    query getAccountTransactionsCount($address: String) {
  account_transactions_aggregate(where: {account_address: {_eq: $address}}) {
    aggregate {
      count
    }
  }
}
    `;

export type SdkFunctionWrapper = <T>(action: (requestHeaders?:Record<string, string>) => Promise<T>, operationName: string, operationType?: string) => Promise<T>;


const defaultWrapper: SdkFunctionWrapper = (action, _operationName, _operationType) => action();

export function getSdk(client: GraphQLClient, withWrapper: SdkFunctionWrapper = defaultWrapper) {
  return {
    getAccountCoinsCount(variables?: Types.GetAccountCoinsCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCoinsCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountCoinsCountQuery>(GetAccountCoinsCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountCoinsCount', 'query');
    },
    getAccountCoinsData(variables: Types.GetAccountCoinsDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCoinsDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountCoinsDataQuery>(GetAccountCoinsData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountCoinsData', 'query');
    },
    getAccountCollectionsWithOwnedTokens(variables: Types.GetAccountCollectionsWithOwnedTokensQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCollectionsWithOwnedTokensQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountCollectionsWithOwnedTokensQuery>(GetAccountCollectionsWithOwnedTokens, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountCollectionsWithOwnedTokens', 'query');
    },
    getAccountOwnedObjects(variables?: Types.GetAccountOwnedObjectsQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountOwnedObjectsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountOwnedObjectsQuery>(GetAccountOwnedObjects, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountOwnedObjects', 'query');
    },
    getAccountOwnedTokens(variables: Types.GetAccountOwnedTokensQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountOwnedTokensQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountOwnedTokensQuery>(GetAccountOwnedTokens, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountOwnedTokens', 'query');
    },
    getAccountOwnedTokensByTokenData(variables: Types.GetAccountOwnedTokensByTokenDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountOwnedTokensByTokenDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountOwnedTokensByTokenDataQuery>(GetAccountOwnedTokensByTokenData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountOwnedTokensByTokenData', 'query');
    },
    getAccountOwnedTokensFromCollection(variables: Types.GetAccountOwnedTokensFromCollectionQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountOwnedTokensFromCollectionQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountOwnedTokensFromCollectionQuery>(GetAccountOwnedTokensFromCollection, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountOwnedTokensFromCollection', 'query');
    },
    getAccountTokensCount(variables?: Types.GetAccountTokensCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountTokensCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountTokensCountQuery>(GetAccountTokensCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountTokensCount', 'query');
    },
    getAccountTransactionsCount(variables?: Types.GetAccountTransactionsCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountTransactionsCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountTransactionsCountQuery>(GetAccountTransactionsCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountTransactionsCount', 'query');
    }
  };
}
export type Sdk = ReturnType<typeof getSdk>;
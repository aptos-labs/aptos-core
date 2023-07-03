import * as Types from './operations';

import { GraphQLClient } from 'graphql-request';
import * as Dom from 'graphql-request/dist/types.dom';
export const CurrentTokenOwnershipFieldsFragmentDoc = `
    fragment CurrentTokenOwnershipFields on current_token_ownerships_v2 {
  token_standard
  is_fungible_v2
  is_soulbound_v2
  property_version_v1
  table_type_v1
  token_properties_mutated_v1
  amount
  last_transaction_timestamp
  last_transaction_version
  storage_id
  owner_address
  current_token_data {
    token_name
    token_data_id
    token_uri
    token_properties
    supply
    maximum
    last_transaction_version
    last_transaction_timestamp
    largest_property_version_v1
    current_collection {
      collection_name
      creator_address
      description
      uri
      collection_id
      last_transaction_version
      current_supply
      mutable_description
      total_minted_v2
      table_handle_v1
      mutable_uri
    }
  }
}
    `;
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
export const GetAccountCoinsData = `
    query getAccountCoinsData($owner_address: String, $offset: Int, $limit: Int) {
  current_coin_balances(
    where: {owner_address: {_eq: $owner_address}}
    offset: $offset
    limit: $limit
  ) {
    amount
    coin_type
    coin_info {
      name
      decimals
      symbol
    }
  }
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
export const GetAccountTokensCount = `
    query getAccountTokensCount($owner_address: String) {
  current_token_ownerships_aggregate(
    where: {owner_address: {_eq: $owner_address}, amount: {_gt: "0"}}
  ) {
    aggregate {
      count
    }
  }
}
    `;
export const GetAccountTransactionsCount = `
    query getAccountTransactionsCount($address: String) {
  move_resources_aggregate(
    where: {address: {_eq: $address}}
    distinct_on: transaction_version
  ) {
    aggregate {
      count
    }
  }
}
    `;
export const GetAccountTransactionsData = `
    query getAccountTransactionsData($address: String, $limit: Int, $offset: Int) {
  move_resources(
    where: {address: {_eq: $address}}
    order_by: {transaction_version: desc}
    distinct_on: transaction_version
    limit: $limit
    offset: $offset
  ) {
    transaction_version
  }
}
    `;
export const GetCollectionData = `
    query getCollectionData($where_condition: current_collections_v2_bool_exp!, $offset: Int, $limit: Int) {
  current_collections_v2(where: $where_condition, offset: $offset, limit: $limit) {
    collection_id
    token_standard
    collection_name
    creator_address
    current_supply
    description
    uri
  }
}
    `;
export const GetCollectionsWithOwnedTokens = `
    query getCollectionsWithOwnedTokens($where_condition: current_collection_ownership_v2_view_bool_exp!, $offset: Int, $limit: Int) {
  current_collection_ownership_v2_view(
    where: $where_condition
    order_by: {last_transaction_version: desc}
    offset: $offset
    limit: $limit
  ) {
    current_collection {
      creator_address
      collection_name
      token_standard
      collection_id
      description
      table_handle_v1
      uri
      total_minted_v2
      max_supply
    }
    distinct_tokens
    last_transaction_version
  }
}
    `;
export const GetDelegatedStakingActivities = `
    query getDelegatedStakingActivities($delegatorAddress: String, $poolAddress: String) {
  delegated_staking_activities(
    where: {delegator_address: {_eq: $delegatorAddress}, pool_address: {_eq: $poolAddress}}
  ) {
    amount
    delegator_address
    event_index
    event_type
    pool_address
    transaction_version
  }
}
    `;
export const GetIndexerLedgerInfo = `
    query getIndexerLedgerInfo {
  ledger_infos {
    chain_id
  }
}
    `;
export const GetNumberOfDelegators = `
    query getNumberOfDelegators($poolAddress: String) {
  num_active_delegator_per_pool(
    where: {pool_address: {_eq: $poolAddress}, num_active_delegator: {_gt: "0"}}
    distinct_on: pool_address
  ) {
    num_active_delegator
  }
}
    `;
export const GetOwnedTokens = `
    query getOwnedTokens($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int) {
  current_token_ownerships_v2(
    where: $where_condition
    offset: $offset
    limit: $limit
  ) {
    ...CurrentTokenOwnershipFields
  }
}
    ${CurrentTokenOwnershipFieldsFragmentDoc}`;
export const GetTokenActivities = `
    query getTokenActivities($idHash: String!, $offset: Int, $limit: Int) {
  token_activities(
    where: {token_data_id_hash: {_eq: $idHash}}
    order_by: {transaction_version: desc}
    offset: $offset
    limit: $limit
  ) {
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
export const GetTokenActivitiesCount = `
    query getTokenActivitiesCount($token_id: String) {
  token_activities_aggregate(where: {token_data_id_hash: {_eq: $token_id}}) {
    aggregate {
      count
    }
  }
}
    `;
export const GetTokenCurrentOwnerData = `
    query getTokenCurrentOwnerData($where_condition: current_token_ownerships_v2_bool_exp!) {
  current_token_ownerships_v2(where: $where_condition) {
    owner_address
  }
}
    `;
export const GetTokenData = `
    query getTokenData($where_condition: current_token_datas_v2_bool_exp) {
  current_token_datas_v2(where: $where_condition) {
    token_data_id
    token_name
    token_uri
    token_properties
    token_standard
    largest_property_version_v1
    maximum
    is_fungible_v2
    supply
    last_transaction_version
    last_transaction_timestamp
    current_collection {
      collection_id
      collection_name
      creator_address
      uri
      current_supply
    }
  }
}
    `;
export const GetTokenOwnedFromCollection = `
    query getTokenOwnedFromCollection($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int) {
  current_token_ownerships_v2(
    where: $where_condition
    offset: $offset
    limit: $limit
  ) {
    ...CurrentTokenOwnershipFields
  }
}
    ${CurrentTokenOwnershipFieldsFragmentDoc}`;
export const GetTokenOwnersData = `
    query getTokenOwnersData($where_condition: current_token_ownerships_v2_bool_exp!) {
  current_token_ownerships_v2(where: $where_condition) {
    owner_address
  }
}
    `;
export const GetTopUserTransactions = `
    query getTopUserTransactions($limit: Int) {
  user_transactions(limit: $limit, order_by: {version: desc}) {
    version
  }
}
    `;
export const GetUserTransactions = `
    query getUserTransactions($limit: Int, $start_version: bigint, $offset: Int) {
  user_transactions(
    limit: $limit
    order_by: {version: desc}
    where: {version: {_lte: $start_version}}
    offset: $offset
  ) {
    version
  }
}
    `;

export type SdkFunctionWrapper = <T>(action: (requestHeaders?:Record<string, string>) => Promise<T>, operationName: string, operationType?: string) => Promise<T>;


const defaultWrapper: SdkFunctionWrapper = (action, _operationName, _operationType) => action();

export function getSdk(client: GraphQLClient, withWrapper: SdkFunctionWrapper = defaultWrapper) {
  return {
    getAccountCoinsData(variables?: Types.GetAccountCoinsDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCoinsDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountCoinsDataQuery>(GetAccountCoinsData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountCoinsData', 'query');
    },
    getAccountCurrentTokens(variables: Types.GetAccountCurrentTokensQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCurrentTokensQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountCurrentTokensQuery>(GetAccountCurrentTokens, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountCurrentTokens', 'query');
    },
    getAccountTokensCount(variables?: Types.GetAccountTokensCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountTokensCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountTokensCountQuery>(GetAccountTokensCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountTokensCount', 'query');
    },
    getAccountTransactionsCount(variables?: Types.GetAccountTransactionsCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountTransactionsCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountTransactionsCountQuery>(GetAccountTransactionsCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountTransactionsCount', 'query');
    },
    getAccountTransactionsData(variables?: Types.GetAccountTransactionsDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountTransactionsDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountTransactionsDataQuery>(GetAccountTransactionsData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountTransactionsData', 'query');
    },
    getCollectionData(variables: Types.GetCollectionDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetCollectionDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetCollectionDataQuery>(GetCollectionData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getCollectionData', 'query');
    },
    getCollectionsWithOwnedTokens(variables: Types.GetCollectionsWithOwnedTokensQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetCollectionsWithOwnedTokensQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetCollectionsWithOwnedTokensQuery>(GetCollectionsWithOwnedTokens, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getCollectionsWithOwnedTokens', 'query');
    },
    getDelegatedStakingActivities(variables?: Types.GetDelegatedStakingActivitiesQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetDelegatedStakingActivitiesQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetDelegatedStakingActivitiesQuery>(GetDelegatedStakingActivities, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getDelegatedStakingActivities', 'query');
    },
    getIndexerLedgerInfo(variables?: Types.GetIndexerLedgerInfoQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetIndexerLedgerInfoQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetIndexerLedgerInfoQuery>(GetIndexerLedgerInfo, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getIndexerLedgerInfo', 'query');
    },
    getNumberOfDelegators(variables?: Types.GetNumberOfDelegatorsQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetNumberOfDelegatorsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetNumberOfDelegatorsQuery>(GetNumberOfDelegators, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getNumberOfDelegators', 'query');
    },
    getOwnedTokens(variables: Types.GetOwnedTokensQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetOwnedTokensQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetOwnedTokensQuery>(GetOwnedTokens, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getOwnedTokens', 'query');
    },
    getTokenActivities(variables: Types.GetTokenActivitiesQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenActivitiesQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenActivitiesQuery>(GetTokenActivities, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenActivities', 'query');
    },
    getTokenActivitiesCount(variables?: Types.GetTokenActivitiesCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenActivitiesCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenActivitiesCountQuery>(GetTokenActivitiesCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenActivitiesCount', 'query');
    },
    getTokenCurrentOwnerData(variables: Types.GetTokenCurrentOwnerDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenCurrentOwnerDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenCurrentOwnerDataQuery>(GetTokenCurrentOwnerData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenCurrentOwnerData', 'query');
    },
    getTokenData(variables?: Types.GetTokenDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenDataQuery>(GetTokenData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenData', 'query');
    },
    getTokenOwnedFromCollection(variables: Types.GetTokenOwnedFromCollectionQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenOwnedFromCollectionQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenOwnedFromCollectionQuery>(GetTokenOwnedFromCollection, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenOwnedFromCollection', 'query');
    },
    getTokenOwnersData(variables: Types.GetTokenOwnersDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenOwnersDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenOwnersDataQuery>(GetTokenOwnersData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenOwnersData', 'query');
    },
    getTopUserTransactions(variables?: Types.GetTopUserTransactionsQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTopUserTransactionsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTopUserTransactionsQuery>(GetTopUserTransactions, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTopUserTransactions', 'query');
    },
    getUserTransactions(variables?: Types.GetUserTransactionsQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetUserTransactionsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetUserTransactionsQuery>(GetUserTransactions, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getUserTransactions', 'query');
    }
  };
}
export type Sdk = ReturnType<typeof getSdk>;
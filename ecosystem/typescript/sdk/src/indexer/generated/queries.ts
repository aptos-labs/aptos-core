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
export const TokenActivitiesFieldsFragmentDoc = `
    fragment TokenActivitiesFields on token_activities_v2 {
  after_value
  before_value
  entry_function_id_str
  event_account_address
  event_index
  from_address
  is_fungible_v2
  property_version_v1
  to_address
  token_amount
  token_data_id
  token_standard
  transaction_timestamp
  transaction_version
  type
}
    `;
export const GetAccountCoinsDataCount = `
    query getAccountCoinsDataCount($address: String) {
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
export const GetAccountTransactionsData = `
    query getAccountTransactionsData($where_condition: account_transactions_bool_exp!, $offset: Int, $limit: Int, $order_by: [account_transactions_order_by!]) {
  account_transactions(
    where: $where_condition
    order_by: $order_by
    limit: $limit
    offset: $offset
  ) {
    token_activities_v2 {
      ...TokenActivitiesFields
    }
    transaction_version
    account_address
  }
}
    ${TokenActivitiesFieldsFragmentDoc}`;
export const GetCollectionData = `
    query getCollectionData($where_condition: current_collections_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_collections_v2_order_by!]) {
  current_collections_v2(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
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
    `;
export const GetCollectionsWithOwnedTokens = `
    query getCollectionsWithOwnedTokens($where_condition: current_collection_ownership_v2_view_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_collection_ownership_v2_view_order_by!]) {
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
export const GetCurrentObjects = `
    query getCurrentObjects($where_condition: current_objects_bool_exp, $offset: Int, $limit: Int, $order_by: [current_objects_order_by!]) {
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
    pool_address
  }
}
    `;
export const GetOwnedTokens = `
    query getOwnedTokens($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
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
export const GetOwnedTokensByTokenData = `
    query getOwnedTokensByTokenData($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
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
export const GetTokenActivities = `
    query getTokenActivities($where_condition: token_activities_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [token_activities_v2_order_by!]) {
  token_activities_v2(
    where: $where_condition
    order_by: $order_by
    offset: $offset
    limit: $limit
  ) {
    ...TokenActivitiesFields
  }
}
    ${TokenActivitiesFieldsFragmentDoc}`;
export const GetTokenActivitiesCount = `
    query getTokenActivitiesCount($token_id: String) {
  token_activities_v2_aggregate(where: {token_data_id: {_eq: $token_id}}) {
    aggregate {
      count
    }
  }
}
    `;
export const GetTokenCurrentOwnerData = `
    query getTokenCurrentOwnerData($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
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
export const GetTokenData = `
    query getTokenData($where_condition: current_token_datas_v2_bool_exp, $offset: Int, $limit: Int, $order_by: [current_token_datas_v2_order_by!]) {
  current_token_datas_v2(
    where: $where_condition
    offset: $offset
    limit: $limit
    order_by: $order_by
  ) {
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
export const GetTokenOwnedFromCollection = `
    query getTokenOwnedFromCollection($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
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
export const GetTokenOwnersData = `
    query getTokenOwnersData($where_condition: current_token_ownerships_v2_bool_exp!, $offset: Int, $limit: Int, $order_by: [current_token_ownerships_v2_order_by!]) {
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
export const GetTopUserTransactions = `
    query getTopUserTransactions($limit: Int) {
  user_transactions(limit: $limit, order_by: {version: desc}) {
    version
  }
}
    `;
export const GetUserTransactions = `
    query getUserTransactions($where_condition: user_transactions_bool_exp!, $offset: Int, $limit: Int, $order_by: [user_transactions_order_by!]) {
  user_transactions(
    order_by: $order_by
    where: $where_condition
    limit: $limit
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
    getAccountCoinsDataCount(variables?: Types.GetAccountCoinsDataCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCoinsDataCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountCoinsDataCountQuery>(GetAccountCoinsDataCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountCoinsDataCount', 'query');
    },
    getAccountCoinsData(variables: Types.GetAccountCoinsDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountCoinsDataQuery> {
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
    getAccountTransactionsData(variables: Types.GetAccountTransactionsDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetAccountTransactionsDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetAccountTransactionsDataQuery>(GetAccountTransactionsData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getAccountTransactionsData', 'query');
    },
    getCollectionData(variables: Types.GetCollectionDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetCollectionDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetCollectionDataQuery>(GetCollectionData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getCollectionData', 'query');
    },
    getCollectionsWithOwnedTokens(variables: Types.GetCollectionsWithOwnedTokensQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetCollectionsWithOwnedTokensQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetCollectionsWithOwnedTokensQuery>(GetCollectionsWithOwnedTokens, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getCollectionsWithOwnedTokens', 'query');
    },
    getCurrentObjects(variables?: Types.GetCurrentObjectsQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetCurrentObjectsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetCurrentObjectsQuery>(GetCurrentObjects, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getCurrentObjects', 'query');
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
    getOwnedTokensByTokenData(variables: Types.GetOwnedTokensByTokenDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetOwnedTokensByTokenDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetOwnedTokensByTokenDataQuery>(GetOwnedTokensByTokenData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getOwnedTokensByTokenData', 'query');
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
    getUserTransactions(variables: Types.GetUserTransactionsQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetUserTransactionsQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetUserTransactionsQuery>(GetUserTransactions, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getUserTransactions', 'query');
    }
  };
}
export type Sdk = ReturnType<typeof getSdk>;
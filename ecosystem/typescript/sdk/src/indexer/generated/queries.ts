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
export const GetCurrentDelegatorBalancesCount = `
    query getCurrentDelegatorBalancesCount($poolAddress: String) {
  current_delegator_balances_aggregate(
    where: {pool_type: {_eq: "active_shares"}, pool_address: {_eq: $poolAddress}, amount: {_gt: "0"}}
    distinct_on: delegator_address
  ) {
    aggregate {
      count
    }
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
export const GetTokenData = `
    query getTokenData($token_id: String) {
  current_token_datas(where: {token_data_id_hash: {_eq: $token_id}}) {
    token_data_id_hash
    name
    collection_name
    creator_address
    default_properties
    largest_property_version
    maximum
    metadata_uri
    payee_address
    royalty_points_denominator
    royalty_points_numerator
    supply
  }
}
    `;
export const GetTokenOwnersData = `
    query getTokenOwnersData($token_id: String, $property_version: numeric) {
  current_token_ownerships(
    where: {token_data_id_hash: {_eq: $token_id}, property_version: {_eq: $property_version}}
  ) {
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
    getCurrentDelegatorBalancesCount(variables?: Types.GetCurrentDelegatorBalancesCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetCurrentDelegatorBalancesCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetCurrentDelegatorBalancesCountQuery>(GetCurrentDelegatorBalancesCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getCurrentDelegatorBalancesCount', 'query');
    },
    getDelegatedStakingActivities(variables?: Types.GetDelegatedStakingActivitiesQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetDelegatedStakingActivitiesQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetDelegatedStakingActivitiesQuery>(GetDelegatedStakingActivities, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getDelegatedStakingActivities', 'query');
    },
    getIndexerLedgerInfo(variables?: Types.GetIndexerLedgerInfoQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetIndexerLedgerInfoQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetIndexerLedgerInfoQuery>(GetIndexerLedgerInfo, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getIndexerLedgerInfo', 'query');
    },
    getTokenActivities(variables: Types.GetTokenActivitiesQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenActivitiesQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenActivitiesQuery>(GetTokenActivities, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenActivities', 'query');
    },
    getTokenActivitiesCount(variables?: Types.GetTokenActivitiesCountQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenActivitiesCountQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenActivitiesCountQuery>(GetTokenActivitiesCount, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenActivitiesCount', 'query');
    },
    getTokenData(variables?: Types.GetTokenDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenDataQuery> {
      return withWrapper((wrappedRequestHeaders) => client.request<Types.GetTokenDataQuery>(GetTokenData, variables, {...requestHeaders, ...wrappedRequestHeaders}), 'getTokenData', 'query');
    },
    getTokenOwnersData(variables?: Types.GetTokenOwnersDataQueryVariables, requestHeaders?: Dom.RequestInit["headers"]): Promise<Types.GetTokenOwnersDataQuery> {
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
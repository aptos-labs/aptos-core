import * as Types from './types';

export type GetAccountCoinsDataQueryVariables = Types.Exact<{
  owner_address?: Types.InputMaybe<Types.Scalars['String']>;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetAccountCoinsDataQuery = { __typename?: 'query_root', current_coin_balances: Array<{ __typename?: 'current_coin_balances', amount: any, coin_type: string, coin_info?: { __typename?: 'coin_infos', name: string, decimals: number, symbol: string } | null }> };

export type GetAccountCurrentTokensQueryVariables = Types.Exact<{
  address: Types.Scalars['String'];
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetAccountCurrentTokensQuery = { __typename?: 'query_root', current_token_ownerships: Array<{ __typename?: 'current_token_ownerships', amount: any, last_transaction_version: any, property_version: any, current_token_data?: { __typename?: 'current_token_datas', creator_address: string, collection_name: string, description: string, metadata_uri: string, name: string, token_data_id_hash: string, collection_data_id_hash: string } | null, current_collection_data?: { __typename?: 'current_collection_datas', metadata_uri: string, supply: any, description: string, collection_name: string, collection_data_id_hash: string, table_handle: string, creator_address: string } | null }> };

export type TokenDataFieldsFragment = { __typename?: 'current_token_datas', creator_address: string, collection_name: string, description: string, metadata_uri: string, name: string, token_data_id_hash: string, collection_data_id_hash: string };

export type CollectionDataFieldsFragment = { __typename?: 'current_collection_datas', metadata_uri: string, supply: any, description: string, collection_name: string, collection_data_id_hash: string, table_handle: string, creator_address: string };

export type GetAccountTokensCountQueryVariables = Types.Exact<{
  owner_address?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetAccountTokensCountQuery = { __typename?: 'query_root', current_token_ownerships_aggregate: { __typename?: 'current_token_ownerships_aggregate', aggregate?: { __typename?: 'current_token_ownerships_aggregate_fields', count: number } | null } };

export type GetAccountTransactionsCountQueryVariables = Types.Exact<{
  address?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetAccountTransactionsCountQuery = { __typename?: 'query_root', move_resources_aggregate: { __typename?: 'move_resources_aggregate', aggregate?: { __typename?: 'move_resources_aggregate_fields', count: number } | null } };

export type GetAccountTransactionsDataQueryVariables = Types.Exact<{
  address?: Types.InputMaybe<Types.Scalars['String']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetAccountTransactionsDataQuery = { __typename?: 'query_root', move_resources: Array<{ __typename?: 'move_resources', transaction_version: any }> };

export type GetCurrentDelegatorBalancesCountQueryVariables = Types.Exact<{
  poolAddress?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetCurrentDelegatorBalancesCountQuery = { __typename?: 'query_root', current_delegator_balances_aggregate: { __typename?: 'current_delegator_balances_aggregate', aggregate?: { __typename?: 'current_delegator_balances_aggregate_fields', count: number } | null } };

export type GetDelegatedStakingActivitiesQueryVariables = Types.Exact<{
  delegatorAddress?: Types.InputMaybe<Types.Scalars['String']>;
  poolAddress?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetDelegatedStakingActivitiesQuery = { __typename?: 'query_root', delegated_staking_activities: Array<{ __typename?: 'delegated_staking_activities', amount: any, delegator_address: string, event_index: any, event_type: string, pool_address: string, transaction_version: any }> };

export type GetIndexerLedgerInfoQueryVariables = Types.Exact<{ [key: string]: never; }>;


export type GetIndexerLedgerInfoQuery = { __typename?: 'query_root', ledger_infos: Array<{ __typename?: 'ledger_infos', chain_id: any }> };

export type GetTokenActivitiesQueryVariables = Types.Exact<{
  idHash: Types.Scalars['String'];
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetTokenActivitiesQuery = { __typename?: 'query_root', token_activities: Array<{ __typename?: 'token_activities', creator_address: string, collection_name: string, name: string, token_data_id_hash: string, collection_data_id_hash: string, from_address?: string | null, to_address?: string | null, transaction_version: any, transaction_timestamp: any, property_version: any, transfer_type: string, event_sequence_number: any, token_amount: any }> };

export type GetTokenActivitiesCountQueryVariables = Types.Exact<{
  token_id?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetTokenActivitiesCountQuery = { __typename?: 'query_root', token_activities_aggregate: { __typename?: 'token_activities_aggregate', aggregate?: { __typename?: 'token_activities_aggregate_fields', count: number } | null } };

export type GetTokenDataQueryVariables = Types.Exact<{
  token_id?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetTokenDataQuery = { __typename?: 'query_root', current_token_datas: Array<{ __typename?: 'current_token_datas', token_data_id_hash: string, name: string, collection_name: string, creator_address: string, default_properties: any, largest_property_version: any, maximum: any, metadata_uri: string, payee_address: string, royalty_points_denominator: any, royalty_points_numerator: any, supply: any }> };

export type GetTokenOwnersDataQueryVariables = Types.Exact<{
  token_id?: Types.InputMaybe<Types.Scalars['String']>;
  property_version?: Types.InputMaybe<Types.Scalars['numeric']>;
}>;


export type GetTokenOwnersDataQuery = { __typename?: 'query_root', current_token_ownerships: Array<{ __typename?: 'current_token_ownerships', owner_address: string }> };

export type GetTopUserTransactionsQueryVariables = Types.Exact<{
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetTopUserTransactionsQuery = { __typename?: 'query_root', user_transactions: Array<{ __typename?: 'user_transactions', version: any }> };

export type GetUserTransactionsQueryVariables = Types.Exact<{
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  start_version?: Types.InputMaybe<Types.Scalars['bigint']>;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetUserTransactionsQuery = { __typename?: 'query_root', user_transactions: Array<{ __typename?: 'user_transactions', version: any }> };

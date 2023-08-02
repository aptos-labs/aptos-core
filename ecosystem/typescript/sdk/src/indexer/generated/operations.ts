import * as Types from './types';

export type CurrentTokenOwnershipFieldsFragment = { __typename?: 'current_token_ownerships_v2', token_standard: string, is_fungible_v2?: boolean | null, is_soulbound_v2?: boolean | null, property_version_v1: any, table_type_v1?: string | null, token_properties_mutated_v1?: any | null, amount: any, last_transaction_timestamp: any, last_transaction_version: any, storage_id: string, owner_address: string, current_token_data?: { __typename?: 'current_token_datas_v2', token_name: string, token_data_id: string, token_uri: string, token_properties: any, supply: any, maximum?: any | null, last_transaction_version: any, last_transaction_timestamp: any, largest_property_version_v1?: any | null, current_collection?: { __typename?: 'current_collections_v2', collection_name: string, creator_address: string, description: string, uri: string, collection_id: string, last_transaction_version: any, current_supply: any, mutable_description?: boolean | null, total_minted_v2?: any | null, table_handle_v1?: string | null, mutable_uri?: boolean | null } | null } | null };

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
  where_condition?: Types.InputMaybe<Types.Current_Token_Ownerships_V2_Bool_Exp>;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetAccountTokensCountQuery = { __typename?: 'query_root', current_token_ownerships_v2_aggregate: { __typename?: 'current_token_ownerships_v2_aggregate', aggregate?: { __typename?: 'current_token_ownerships_v2_aggregate_fields', count: number } | null } };

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

export type GetCollectionDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Collections_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Collections_V2_Order_By> | Types.Current_Collections_V2_Order_By>;
}>;


export type GetCollectionDataQuery = { __typename?: 'query_root', current_collections_v2: Array<{ __typename?: 'current_collections_v2', collection_id: string, token_standard: string, collection_name: string, creator_address: string, current_supply: any, description: string, uri: string }> };

export type GetCollectionsWithOwnedTokensQueryVariables = Types.Exact<{
  where_condition: Types.Current_Collection_Ownership_V2_View_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Collection_Ownership_V2_View_Order_By> | Types.Current_Collection_Ownership_V2_View_Order_By>;
}>;


export type GetCollectionsWithOwnedTokensQuery = { __typename?: 'query_root', current_collection_ownership_v2_view: Array<{ __typename?: 'current_collection_ownership_v2_view', distinct_tokens?: any | null, last_transaction_version?: any | null, current_collection?: { __typename?: 'current_collections_v2', creator_address: string, collection_name: string, token_standard: string, collection_id: string, description: string, table_handle_v1?: string | null, uri: string, total_minted_v2?: any | null, max_supply?: any | null } | null }> };

export type GetDelegatedStakingActivitiesQueryVariables = Types.Exact<{
  delegatorAddress?: Types.InputMaybe<Types.Scalars['String']>;
  poolAddress?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetDelegatedStakingActivitiesQuery = { __typename?: 'query_root', delegated_staking_activities: Array<{ __typename?: 'delegated_staking_activities', amount: any, delegator_address: string, event_index: any, event_type: string, pool_address: string, transaction_version: any }> };

export type GetIndexerLedgerInfoQueryVariables = Types.Exact<{ [key: string]: never; }>;


export type GetIndexerLedgerInfoQuery = { __typename?: 'query_root', ledger_infos: Array<{ __typename?: 'ledger_infos', chain_id: any }> };

export type GetNumberOfDelegatorsQueryVariables = Types.Exact<{
  poolAddress?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetNumberOfDelegatorsQuery = { __typename?: 'query_root', num_active_delegator_per_pool: Array<{ __typename?: 'num_active_delegator_per_pool', num_active_delegator?: any | null }> };

export type GetOwnedTokensQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetOwnedTokensQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, is_fungible_v2?: boolean | null, is_soulbound_v2?: boolean | null, property_version_v1: any, table_type_v1?: string | null, token_properties_mutated_v1?: any | null, amount: any, last_transaction_timestamp: any, last_transaction_version: any, storage_id: string, owner_address: string, current_token_data?: { __typename?: 'current_token_datas_v2', token_name: string, token_data_id: string, token_uri: string, token_properties: any, supply: any, maximum?: any | null, last_transaction_version: any, last_transaction_timestamp: any, largest_property_version_v1?: any | null, current_collection?: { __typename?: 'current_collections_v2', collection_name: string, creator_address: string, description: string, uri: string, collection_id: string, last_transaction_version: any, current_supply: any, mutable_description?: boolean | null, total_minted_v2?: any | null, table_handle_v1?: string | null, mutable_uri?: boolean | null } | null } | null }> };

export type GetOwnedTokensByTokenDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetOwnedTokensByTokenDataQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, is_fungible_v2?: boolean | null, is_soulbound_v2?: boolean | null, property_version_v1: any, table_type_v1?: string | null, token_properties_mutated_v1?: any | null, amount: any, last_transaction_timestamp: any, last_transaction_version: any, storage_id: string, owner_address: string, current_token_data?: { __typename?: 'current_token_datas_v2', token_name: string, token_data_id: string, token_uri: string, token_properties: any, supply: any, maximum?: any | null, last_transaction_version: any, last_transaction_timestamp: any, largest_property_version_v1?: any | null, current_collection?: { __typename?: 'current_collections_v2', collection_name: string, creator_address: string, description: string, uri: string, collection_id: string, last_transaction_version: any, current_supply: any, mutable_description?: boolean | null, total_minted_v2?: any | null, table_handle_v1?: string | null, mutable_uri?: boolean | null } | null } | null }> };

export type GetTokenActivitiesQueryVariables = Types.Exact<{
  where_condition: Types.Token_Activities_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Token_Activities_V2_Order_By> | Types.Token_Activities_V2_Order_By>;
}>;


export type GetTokenActivitiesQuery = { __typename?: 'query_root', token_activities_v2: Array<{ __typename?: 'token_activities_v2', after_value?: string | null, before_value?: string | null, entry_function_id_str?: string | null, event_account_address: string, event_index: any, from_address?: string | null, is_fungible_v2?: boolean | null, property_version_v1: any, to_address?: string | null, token_amount: any, token_data_id: string, token_standard: string, transaction_timestamp: any, transaction_version: any, type: string }> };

export type GetTokenActivitiesCountQueryVariables = Types.Exact<{
  token_id?: Types.InputMaybe<Types.Scalars['String']>;
}>;


export type GetTokenActivitiesCountQuery = { __typename?: 'query_root', token_activities_v2_aggregate: { __typename?: 'token_activities_v2_aggregate', aggregate?: { __typename?: 'token_activities_v2_aggregate_fields', count: number } | null } };

export type GetTokenCurrentOwnerDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetTokenCurrentOwnerDataQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', owner_address: string }> };

export type GetTokenDataQueryVariables = Types.Exact<{
  where_condition?: Types.InputMaybe<Types.Current_Token_Datas_V2_Bool_Exp>;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Datas_V2_Order_By> | Types.Current_Token_Datas_V2_Order_By>;
}>;


export type GetTokenDataQuery = { __typename?: 'query_root', current_token_datas_v2: Array<{ __typename?: 'current_token_datas_v2', token_data_id: string, token_name: string, token_uri: string, token_properties: any, token_standard: string, largest_property_version_v1?: any | null, maximum?: any | null, is_fungible_v2?: boolean | null, supply: any, last_transaction_version: any, last_transaction_timestamp: any, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, uri: string, current_supply: any } | null }> };

export type GetTokenOwnedFromCollectionQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetTokenOwnedFromCollectionQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, is_fungible_v2?: boolean | null, is_soulbound_v2?: boolean | null, property_version_v1: any, table_type_v1?: string | null, token_properties_mutated_v1?: any | null, amount: any, last_transaction_timestamp: any, last_transaction_version: any, storage_id: string, owner_address: string, current_token_data?: { __typename?: 'current_token_datas_v2', token_name: string, token_data_id: string, token_uri: string, token_properties: any, supply: any, maximum?: any | null, last_transaction_version: any, last_transaction_timestamp: any, largest_property_version_v1?: any | null, current_collection?: { __typename?: 'current_collections_v2', collection_name: string, creator_address: string, description: string, uri: string, collection_id: string, last_transaction_version: any, current_supply: any, mutable_description?: boolean | null, total_minted_v2?: any | null, table_handle_v1?: string | null, mutable_uri?: boolean | null } | null } | null }> };

export type GetTokenOwnersDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetTokenOwnersDataQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', owner_address: string }> };

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

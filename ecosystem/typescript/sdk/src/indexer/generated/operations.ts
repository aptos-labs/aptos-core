import * as Types from './types';

export type CurrentTokenOwnershipFieldsFragment = { __typename?: 'current_token_ownerships_v2', token_standard: string, token_properties_mutated_v1?: any | null, token_data_id: string, table_type_v1?: string | null, storage_id: string, property_version_v1: any, owner_address: string, last_transaction_version: any, last_transaction_timestamp: any, is_soulbound_v2?: boolean | null, is_fungible_v2?: boolean | null, amount: any, current_token_data?: { __typename?: 'current_token_datas_v2', collection_id: string, description: string, is_fungible_v2?: boolean | null, largest_property_version_v1?: any | null, last_transaction_timestamp: any, last_transaction_version: any, maximum?: any | null, supply: any, token_data_id: string, token_name: string, token_properties: any, token_standard: string, token_uri: string, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null } | null };

export type GetAccountCoinsDataCountQueryVariables = Types.Exact<{
  address?: Types.InputMaybe<Types.Scalars['String']['input']>;
}>;


export type GetAccountCoinsDataCountQuery = { __typename?: 'query_root', current_fungible_asset_balances_aggregate: { __typename?: 'current_fungible_asset_balances_aggregate', aggregate?: { __typename?: 'current_fungible_asset_balances_aggregate_fields', count: number } | null } };

export type GetAccountCoinsDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Fungible_Asset_Balances_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Fungible_Asset_Balances_Order_By> | Types.Current_Fungible_Asset_Balances_Order_By>;
}>;


export type GetAccountCoinsDataQuery = { __typename?: 'query_root', current_fungible_asset_balances: Array<{ __typename?: 'current_fungible_asset_balances', amount: any, asset_type: string, is_frozen: boolean, is_primary: boolean, last_transaction_timestamp: any, last_transaction_version: any, owner_address: string, storage_id: string, token_standard: string, metadata?: { __typename?: 'fungible_asset_metadata', token_standard: string, symbol: string, supply_aggregator_table_key_v1?: string | null, supply_aggregator_table_handle_v1?: string | null, project_uri?: string | null, name: string, last_transaction_version: any, last_transaction_timestamp: any, icon_uri?: string | null, decimals: number, creator_address: string, asset_type: string } | null }> };

export type GetAccountCurrentTokensQueryVariables = Types.Exact<{
  address: Types.Scalars['String']['input'];
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
}>;


export type GetAccountCurrentTokensQuery = { __typename?: 'query_root', current_token_ownerships: Array<{ __typename?: 'current_token_ownerships', amount: any, last_transaction_version: any, property_version: any, current_token_data?: { __typename?: 'current_token_datas', creator_address: string, collection_name: string, description: string, metadata_uri: string, name: string, token_data_id_hash: string, collection_data_id_hash: string } | null, current_collection_data?: { __typename?: 'current_collection_datas', metadata_uri: string, supply: any, description: string, collection_name: string, collection_data_id_hash: string, table_handle: string, creator_address: string } | null }> };

export type TokenDataFieldsFragment = { __typename?: 'current_token_datas', creator_address: string, collection_name: string, description: string, metadata_uri: string, name: string, token_data_id_hash: string, collection_data_id_hash: string };

export type CollectionDataFieldsFragment = { __typename?: 'current_collection_datas', metadata_uri: string, supply: any, description: string, collection_name: string, collection_data_id_hash: string, table_handle: string, creator_address: string };

export type GetAccountTokensCountQueryVariables = Types.Exact<{
  where_condition?: Types.InputMaybe<Types.Current_Token_Ownerships_V2_Bool_Exp>;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
}>;


export type GetAccountTokensCountQuery = { __typename?: 'query_root', current_token_ownerships_v2_aggregate: { __typename?: 'current_token_ownerships_v2_aggregate', aggregate?: { __typename?: 'current_token_ownerships_v2_aggregate_fields', count: number } | null } };

export type GetAccountTransactionsCountQueryVariables = Types.Exact<{
  address?: Types.InputMaybe<Types.Scalars['String']['input']>;
}>;


export type GetAccountTransactionsCountQuery = { __typename?: 'query_root', account_transactions_aggregate: { __typename?: 'account_transactions_aggregate', aggregate?: { __typename?: 'account_transactions_aggregate_fields', count: number } | null } };

export type GetAccountTransactionsDataQueryVariables = Types.Exact<{
  where_condition: Types.Account_Transactions_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Account_Transactions_Order_By> | Types.Account_Transactions_Order_By>;
}>;


export type GetAccountTransactionsDataQuery = { __typename?: 'query_root', account_transactions: Array<{ __typename?: 'account_transactions', transaction_version: any, account_address: string, token_activities_v2: Array<{ __typename?: 'token_activities_v2', after_value?: string | null, before_value?: string | null, entry_function_id_str?: string | null, event_account_address: string, event_index: any, from_address?: string | null, is_fungible_v2?: boolean | null, property_version_v1: any, to_address?: string | null, token_amount: any, token_data_id: string, token_standard: string, transaction_timestamp: any, transaction_version: any, type: string }> }> };

export type GetCollectionDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Collections_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Collections_V2_Order_By> | Types.Current_Collections_V2_Order_By>;
}>;


export type GetCollectionDataQuery = { __typename?: 'query_root', current_collections_v2: Array<{ __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string }> };

export type GetCollectionsWithOwnedTokensQueryVariables = Types.Exact<{
  where_condition: Types.Current_Collection_Ownership_V2_View_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Collection_Ownership_V2_View_Order_By> | Types.Current_Collection_Ownership_V2_View_Order_By>;
}>;


export type GetCollectionsWithOwnedTokensQuery = { __typename?: 'query_root', current_collection_ownership_v2_view: Array<{ __typename?: 'current_collection_ownership_v2_view', collection_id?: string | null, collection_name?: string | null, collection_uri?: string | null, creator_address?: string | null, distinct_tokens?: any | null, last_transaction_version?: any | null, owner_address?: string | null, single_token_uri?: string | null, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, mutable_description?: boolean | null, max_supply?: any | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null }> };

export type GetCurrentObjectsQueryVariables = Types.Exact<{
  where_condition?: Types.InputMaybe<Types.Current_Objects_Bool_Exp>;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Objects_Order_By> | Types.Current_Objects_Order_By>;
}>;


export type GetCurrentObjectsQuery = { __typename?: 'query_root', current_objects: Array<{ __typename?: 'current_objects', allow_ungated_transfer: boolean, state_key_hash: string, owner_address: string, object_address: string, last_transaction_version: any, last_guid_creation_num: any, is_deleted: boolean }> };

export type GetDelegatedStakingActivitiesQueryVariables = Types.Exact<{
  delegatorAddress?: Types.InputMaybe<Types.Scalars['String']['input']>;
  poolAddress?: Types.InputMaybe<Types.Scalars['String']['input']>;
}>;


export type GetDelegatedStakingActivitiesQuery = { __typename?: 'query_root', delegated_staking_activities: Array<{ __typename?: 'delegated_staking_activities', amount: any, delegator_address: string, event_index: any, event_type: string, pool_address: string, transaction_version: any }> };

export type GetIndexerLedgerInfoQueryVariables = Types.Exact<{ [key: string]: never; }>;


export type GetIndexerLedgerInfoQuery = { __typename?: 'query_root', ledger_infos: Array<{ __typename?: 'ledger_infos', chain_id: any }> };

export type GetNumberOfDelegatorsQueryVariables = Types.Exact<{
  poolAddress?: Types.InputMaybe<Types.Scalars['String']['input']>;
}>;


export type GetNumberOfDelegatorsQuery = { __typename?: 'query_root', num_active_delegator_per_pool: Array<{ __typename?: 'num_active_delegator_per_pool', num_active_delegator?: any | null, pool_address?: string | null }> };

export type GetOwnedTokensQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetOwnedTokensQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, token_properties_mutated_v1?: any | null, token_data_id: string, table_type_v1?: string | null, storage_id: string, property_version_v1: any, owner_address: string, last_transaction_version: any, last_transaction_timestamp: any, is_soulbound_v2?: boolean | null, is_fungible_v2?: boolean | null, amount: any, current_token_data?: { __typename?: 'current_token_datas_v2', collection_id: string, description: string, is_fungible_v2?: boolean | null, largest_property_version_v1?: any | null, last_transaction_timestamp: any, last_transaction_version: any, maximum?: any | null, supply: any, token_data_id: string, token_name: string, token_properties: any, token_standard: string, token_uri: string, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null } | null }> };

export type GetOwnedTokensByTokenDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetOwnedTokensByTokenDataQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, token_properties_mutated_v1?: any | null, token_data_id: string, table_type_v1?: string | null, storage_id: string, property_version_v1: any, owner_address: string, last_transaction_version: any, last_transaction_timestamp: any, is_soulbound_v2?: boolean | null, is_fungible_v2?: boolean | null, amount: any, current_token_data?: { __typename?: 'current_token_datas_v2', collection_id: string, description: string, is_fungible_v2?: boolean | null, largest_property_version_v1?: any | null, last_transaction_timestamp: any, last_transaction_version: any, maximum?: any | null, supply: any, token_data_id: string, token_name: string, token_properties: any, token_standard: string, token_uri: string, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null } | null }> };

export type GetTokenActivitiesQueryVariables = Types.Exact<{
  where_condition: Types.Token_Activities_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Token_Activities_V2_Order_By> | Types.Token_Activities_V2_Order_By>;
}>;


export type GetTokenActivitiesQuery = { __typename?: 'query_root', token_activities_v2: Array<{ __typename?: 'token_activities_v2', after_value?: string | null, before_value?: string | null, entry_function_id_str?: string | null, event_account_address: string, event_index: any, from_address?: string | null, is_fungible_v2?: boolean | null, property_version_v1: any, to_address?: string | null, token_amount: any, token_data_id: string, token_standard: string, transaction_timestamp: any, transaction_version: any, type: string }> };

export type GetTokenActivitiesCountQueryVariables = Types.Exact<{
  token_id?: Types.InputMaybe<Types.Scalars['String']['input']>;
}>;


export type GetTokenActivitiesCountQuery = { __typename?: 'query_root', token_activities_v2_aggregate: { __typename?: 'token_activities_v2_aggregate', aggregate?: { __typename?: 'token_activities_v2_aggregate_fields', count: number } | null } };

export type GetTokenCurrentOwnerDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetTokenCurrentOwnerDataQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, token_properties_mutated_v1?: any | null, token_data_id: string, table_type_v1?: string | null, storage_id: string, property_version_v1: any, owner_address: string, last_transaction_version: any, last_transaction_timestamp: any, is_soulbound_v2?: boolean | null, is_fungible_v2?: boolean | null, amount: any, current_token_data?: { __typename?: 'current_token_datas_v2', collection_id: string, description: string, is_fungible_v2?: boolean | null, largest_property_version_v1?: any | null, last_transaction_timestamp: any, last_transaction_version: any, maximum?: any | null, supply: any, token_data_id: string, token_name: string, token_properties: any, token_standard: string, token_uri: string, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null } | null }> };

export type GetTokenDataQueryVariables = Types.Exact<{
  where_condition?: Types.InputMaybe<Types.Current_Token_Datas_V2_Bool_Exp>;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Datas_V2_Order_By> | Types.Current_Token_Datas_V2_Order_By>;
}>;


export type GetTokenDataQuery = { __typename?: 'query_root', current_token_datas_v2: Array<{ __typename?: 'current_token_datas_v2', collection_id: string, description: string, is_fungible_v2?: boolean | null, largest_property_version_v1?: any | null, last_transaction_timestamp: any, last_transaction_version: any, maximum?: any | null, supply: any, token_data_id: string, token_name: string, token_properties: any, token_standard: string, token_uri: string, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null }> };

export type GetTokenOwnedFromCollectionQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetTokenOwnedFromCollectionQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, token_properties_mutated_v1?: any | null, token_data_id: string, table_type_v1?: string | null, storage_id: string, property_version_v1: any, owner_address: string, last_transaction_version: any, last_transaction_timestamp: any, is_soulbound_v2?: boolean | null, is_fungible_v2?: boolean | null, amount: any, current_token_data?: { __typename?: 'current_token_datas_v2', collection_id: string, description: string, is_fungible_v2?: boolean | null, largest_property_version_v1?: any | null, last_transaction_timestamp: any, last_transaction_version: any, maximum?: any | null, supply: any, token_data_id: string, token_name: string, token_properties: any, token_standard: string, token_uri: string, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null } | null }> };

export type GetTokenOwnersDataQueryVariables = Types.Exact<{
  where_condition: Types.Current_Token_Ownerships_V2_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.Current_Token_Ownerships_V2_Order_By> | Types.Current_Token_Ownerships_V2_Order_By>;
}>;


export type GetTokenOwnersDataQuery = { __typename?: 'query_root', current_token_ownerships_v2: Array<{ __typename?: 'current_token_ownerships_v2', token_standard: string, token_properties_mutated_v1?: any | null, token_data_id: string, table_type_v1?: string | null, storage_id: string, property_version_v1: any, owner_address: string, last_transaction_version: any, last_transaction_timestamp: any, is_soulbound_v2?: boolean | null, is_fungible_v2?: boolean | null, amount: any, current_token_data?: { __typename?: 'current_token_datas_v2', collection_id: string, description: string, is_fungible_v2?: boolean | null, largest_property_version_v1?: any | null, last_transaction_timestamp: any, last_transaction_version: any, maximum?: any | null, supply: any, token_data_id: string, token_name: string, token_properties: any, token_standard: string, token_uri: string, current_collection?: { __typename?: 'current_collections_v2', collection_id: string, collection_name: string, creator_address: string, current_supply: any, description: string, last_transaction_timestamp: any, last_transaction_version: any, max_supply?: any | null, mutable_description?: boolean | null, mutable_uri?: boolean | null, table_handle_v1?: string | null, token_standard: string, total_minted_v2?: any | null, uri: string } | null } | null }> };

export type GetTopUserTransactionsQueryVariables = Types.Exact<{
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
}>;


export type GetTopUserTransactionsQuery = { __typename?: 'query_root', user_transactions: Array<{ __typename?: 'user_transactions', version: any }> };

export type GetUserTransactionsQueryVariables = Types.Exact<{
  where_condition: Types.User_Transactions_Bool_Exp;
  offset?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']['input']>;
  order_by?: Types.InputMaybe<Array<Types.User_Transactions_Order_By> | Types.User_Transactions_Order_By>;
}>;


export type GetUserTransactionsQuery = { __typename?: 'query_root', user_transactions: Array<{ __typename?: 'user_transactions', version: any }> };

export type TokenActivitiesFieldsFragment = { __typename?: 'token_activities_v2', after_value?: string | null, before_value?: string | null, entry_function_id_str?: string | null, event_account_address: string, event_index: any, from_address?: string | null, is_fungible_v2?: boolean | null, property_version_v1: any, to_address?: string | null, token_amount: any, token_data_id: string, token_standard: string, transaction_timestamp: any, transaction_version: any, type: string };

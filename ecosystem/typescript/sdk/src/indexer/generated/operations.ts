import * as Types from './types';

export type GetAccountCurrentTokensQueryVariables = Types.Exact<{
  address: Types.Scalars['String'];
  offset?: Types.InputMaybe<Types.Scalars['Int']>;
  limit?: Types.InputMaybe<Types.Scalars['Int']>;
}>;


export type GetAccountCurrentTokensQuery = { __typename?: 'query_root', current_token_ownerships: Array<{ __typename?: 'current_token_ownerships', amount: any, last_transaction_version: any, property_version: any, current_token_data?: { __typename?: 'current_token_datas', creator_address: string, collection_name: string, description: string, metadata_uri: string, name: string, token_data_id_hash: string, collection_data_id_hash: string } | null, current_collection_data?: { __typename?: 'current_collection_datas', metadata_uri: string, supply: any, description: string, collection_name: string, collection_data_id_hash: string, table_handle: string, creator_address: string } | null }> };

export type TokenDataFieldsFragment = { __typename?: 'current_token_datas', creator_address: string, collection_name: string, description: string, metadata_uri: string, name: string, token_data_id_hash: string, collection_data_id_hash: string };

export type CollectionDataFieldsFragment = { __typename?: 'current_collection_datas', metadata_uri: string, supply: any, description: string, collection_name: string, collection_data_id_hash: string, table_handle: string, creator_address: string };

export type GetTokenActivitiesQueryVariables = Types.Exact<{
  idHash: Types.Scalars['String'];
}>;


export type GetTokenActivitiesQuery = { __typename?: 'query_root', token_activities: Array<{ __typename?: 'token_activities', creator_address: string, collection_name: string, name: string, token_data_id_hash: string, collection_data_id_hash: string, from_address?: string | null, to_address?: string | null, transaction_version: any, transaction_timestamp: any, property_version: any, transfer_type: string, event_sequence_number: any, token_amount: any }> };

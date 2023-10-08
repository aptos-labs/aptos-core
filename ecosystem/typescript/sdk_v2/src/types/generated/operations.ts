import * as Types from "./types";

export type CurrentTokenOwnershipFieldsFragment = {
  token_standard: string;
  token_properties_mutated_v1?: any | null;
  token_data_id: string;
  table_type_v1?: string | null;
  storage_id: string;
  property_version_v1: any;
  owner_address: string;
  last_transaction_version: any;
  last_transaction_timestamp: any;
  is_soulbound_v2?: boolean | null;
  is_fungible_v2?: boolean | null;
  amount: any;
  current_token_data?: {
    collection_id: string;
    description: string;
    is_fungible_v2?: boolean | null;
    largest_property_version_v1?: any | null;
    last_transaction_timestamp: any;
    last_transaction_version: any;
    maximum?: any | null;
    supply: any;
    token_data_id: string;
    token_name: string;
    token_properties: any;
    token_standard: string;
    token_uri: string;
    current_collection?: {
      collection_id: string;
      collection_name: string;
      creator_address: string;
      current_supply: any;
      description: string;
      last_transaction_timestamp: any;
      last_transaction_version: any;
      max_supply?: any | null;
      mutable_description?: boolean | null;
      mutable_uri?: boolean | null;
      table_handle_v1?: string | null;
      token_standard: string;
      total_minted_v2?: any | null;
      uri: string;
    } | null;
  } | null;
};

export type GetAccountCoinsCountQueryVariables = Types.Exact<{
  address?: Types.InputMaybe<Types.Scalars["String"]>;
}>;

export type GetAccountCoinsCountQuery = {
  current_fungible_asset_balances_aggregate: { aggregate?: { count: number } | null };
};

export type GetAccountCoinsDataQueryVariables = Types.Exact<{
  where_condition: Types.CurrentFungibleAssetBalancesBoolExp;
  offset?: Types.InputMaybe<Types.Scalars["Int"]>;
  limit?: Types.InputMaybe<Types.Scalars["Int"]>;
  order_by?: Types.InputMaybe<
    Array<Types.CurrentFungibleAssetBalancesOrderBy> | Types.CurrentFungibleAssetBalancesOrderBy
  >;
}>;

export type GetAccountCoinsDataQuery = {
  current_fungible_asset_balances: Array<{
    amount: any;
    asset_type: string;
    is_frozen: boolean;
    is_primary: boolean;
    last_transaction_timestamp: any;
    last_transaction_version: any;
    owner_address: string;
    storage_id: string;
    token_standard: string;
    metadata?: {
      token_standard: string;
      symbol: string;
      supply_aggregator_table_key_v1?: string | null;
      supply_aggregator_table_handle_v1?: string | null;
      project_uri?: string | null;
      name: string;
      last_transaction_version: any;
      last_transaction_timestamp: any;
      icon_uri?: string | null;
      decimals: number;
      creator_address: string;
      asset_type: string;
    } | null;
  }>;
};

export type GetAccountCollectionsWithOwnedTokensQueryVariables = Types.Exact<{
  where_condition: Types.CurrentCollectionOwnershipV2ViewBoolExp;
  offset?: Types.InputMaybe<Types.Scalars["Int"]>;
  limit?: Types.InputMaybe<Types.Scalars["Int"]>;
  order_by?: Types.InputMaybe<
    Array<Types.CurrentCollectionOwnershipV2ViewOrderBy> | Types.CurrentCollectionOwnershipV2ViewOrderBy
  >;
}>;

export type GetAccountCollectionsWithOwnedTokensQuery = {
  current_collection_ownership_v2_view: Array<{
    collection_id?: string | null;
    collection_name?: string | null;
    collection_uri?: string | null;
    creator_address?: string | null;
    distinct_tokens?: any | null;
    last_transaction_version?: any | null;
    owner_address?: string | null;
    single_token_uri?: string | null;
    current_collection?: {
      collection_id: string;
      collection_name: string;
      creator_address: string;
      current_supply: any;
      description: string;
      last_transaction_timestamp: any;
      last_transaction_version: any;
      mutable_description?: boolean | null;
      max_supply?: any | null;
      mutable_uri?: boolean | null;
      table_handle_v1?: string | null;
      token_standard: string;
      total_minted_v2?: any | null;
      uri: string;
    } | null;
  }>;
};

export type GetAccountOwnedObjectsQueryVariables = Types.Exact<{
  where_condition?: Types.InputMaybe<Types.CurrentObjectsBoolExp>;
  offset?: Types.InputMaybe<Types.Scalars["Int"]>;
  limit?: Types.InputMaybe<Types.Scalars["Int"]>;
  order_by?: Types.InputMaybe<Array<Types.CurrentObjectsOrderBy> | Types.CurrentObjectsOrderBy>;
}>;

export type GetAccountOwnedObjectsQuery = {
  current_objects: Array<{
    allow_ungated_transfer: boolean;
    state_key_hash: string;
    owner_address: string;
    object_address: string;
    last_transaction_version: any;
    last_guid_creation_num: any;
    is_deleted: boolean;
  }>;
};

export type GetAccountOwnedTokensQueryVariables = Types.Exact<{
  where_condition: Types.CurrentTokenOwnershipsV2BoolExp;
  offset?: Types.InputMaybe<Types.Scalars["Int"]>;
  limit?: Types.InputMaybe<Types.Scalars["Int"]>;
  order_by?: Types.InputMaybe<Array<Types.CurrentTokenOwnershipsV2OrderBy> | Types.CurrentTokenOwnershipsV2OrderBy>;
}>;

export type GetAccountOwnedTokensQuery = {
  current_token_ownerships_v2: Array<{
    token_standard: string;
    token_properties_mutated_v1?: any | null;
    token_data_id: string;
    table_type_v1?: string | null;
    storage_id: string;
    property_version_v1: any;
    owner_address: string;
    last_transaction_version: any;
    last_transaction_timestamp: any;
    is_soulbound_v2?: boolean | null;
    is_fungible_v2?: boolean | null;
    amount: any;
    current_token_data?: {
      collection_id: string;
      description: string;
      is_fungible_v2?: boolean | null;
      largest_property_version_v1?: any | null;
      last_transaction_timestamp: any;
      last_transaction_version: any;
      maximum?: any | null;
      supply: any;
      token_data_id: string;
      token_name: string;
      token_properties: any;
      token_standard: string;
      token_uri: string;
      current_collection?: {
        collection_id: string;
        collection_name: string;
        creator_address: string;
        current_supply: any;
        description: string;
        last_transaction_timestamp: any;
        last_transaction_version: any;
        max_supply?: any | null;
        mutable_description?: boolean | null;
        mutable_uri?: boolean | null;
        table_handle_v1?: string | null;
        token_standard: string;
        total_minted_v2?: any | null;
        uri: string;
      } | null;
    } | null;
  }>;
};

export type GetAccountOwnedTokensByTokenDataQueryVariables = Types.Exact<{
  where_condition: Types.CurrentTokenOwnershipsV2BoolExp;
  offset?: Types.InputMaybe<Types.Scalars["Int"]>;
  limit?: Types.InputMaybe<Types.Scalars["Int"]>;
  order_by?: Types.InputMaybe<Array<Types.CurrentTokenOwnershipsV2OrderBy> | Types.CurrentTokenOwnershipsV2OrderBy>;
}>;

export type GetAccountOwnedTokensByTokenDataQuery = {
  current_token_ownerships_v2: Array<{
    token_standard: string;
    token_properties_mutated_v1?: any | null;
    token_data_id: string;
    table_type_v1?: string | null;
    storage_id: string;
    property_version_v1: any;
    owner_address: string;
    last_transaction_version: any;
    last_transaction_timestamp: any;
    is_soulbound_v2?: boolean | null;
    is_fungible_v2?: boolean | null;
    amount: any;
    current_token_data?: {
      collection_id: string;
      description: string;
      is_fungible_v2?: boolean | null;
      largest_property_version_v1?: any | null;
      last_transaction_timestamp: any;
      last_transaction_version: any;
      maximum?: any | null;
      supply: any;
      token_data_id: string;
      token_name: string;
      token_properties: any;
      token_standard: string;
      token_uri: string;
      current_collection?: {
        collection_id: string;
        collection_name: string;
        creator_address: string;
        current_supply: any;
        description: string;
        last_transaction_timestamp: any;
        last_transaction_version: any;
        max_supply?: any | null;
        mutable_description?: boolean | null;
        mutable_uri?: boolean | null;
        table_handle_v1?: string | null;
        token_standard: string;
        total_minted_v2?: any | null;
        uri: string;
      } | null;
    } | null;
  }>;
};

export type GetAccountOwnedTokensFromCollectionQueryVariables = Types.Exact<{
  where_condition: Types.CurrentTokenOwnershipsV2BoolExp;
  offset?: Types.InputMaybe<Types.Scalars["Int"]>;
  limit?: Types.InputMaybe<Types.Scalars["Int"]>;
  order_by?: Types.InputMaybe<Array<Types.CurrentTokenOwnershipsV2OrderBy> | Types.CurrentTokenOwnershipsV2OrderBy>;
}>;

export type GetAccountOwnedTokensFromCollectionQuery = {
  current_token_ownerships_v2: Array<{
    token_standard: string;
    token_properties_mutated_v1?: any | null;
    token_data_id: string;
    table_type_v1?: string | null;
    storage_id: string;
    property_version_v1: any;
    owner_address: string;
    last_transaction_version: any;
    last_transaction_timestamp: any;
    is_soulbound_v2?: boolean | null;
    is_fungible_v2?: boolean | null;
    amount: any;
    current_token_data?: {
      collection_id: string;
      description: string;
      is_fungible_v2?: boolean | null;
      largest_property_version_v1?: any | null;
      last_transaction_timestamp: any;
      last_transaction_version: any;
      maximum?: any | null;
      supply: any;
      token_data_id: string;
      token_name: string;
      token_properties: any;
      token_standard: string;
      token_uri: string;
      current_collection?: {
        collection_id: string;
        collection_name: string;
        creator_address: string;
        current_supply: any;
        description: string;
        last_transaction_timestamp: any;
        last_transaction_version: any;
        max_supply?: any | null;
        mutable_description?: boolean | null;
        mutable_uri?: boolean | null;
        table_handle_v1?: string | null;
        token_standard: string;
        total_minted_v2?: any | null;
        uri: string;
      } | null;
    } | null;
  }>;
};

export type GetAccountTokensCountQueryVariables = Types.Exact<{
  where_condition?: Types.InputMaybe<Types.CurrentTokenOwnershipsV2BoolExp>;
  offset?: Types.InputMaybe<Types.Scalars["Int"]>;
  limit?: Types.InputMaybe<Types.Scalars["Int"]>;
}>;

export type GetAccountTokensCountQuery = {
  current_token_ownerships_v2_aggregate: { aggregate?: { count: number } | null };
};

export type GetAccountTransactionsCountQueryVariables = Types.Exact<{
  address?: Types.InputMaybe<Types.Scalars["String"]>;
}>;

export type GetAccountTransactionsCountQuery = {
  account_transactions_aggregate: { aggregate?: { count: number } | null };
};

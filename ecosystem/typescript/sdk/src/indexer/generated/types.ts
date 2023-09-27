export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: string;
  String: string;
  Boolean: boolean;
  Int: number;
  Float: number;
  bigint: any;
  jsonb: any;
  numeric: any;
  timestamp: any;
  timestamptz: any;
};

/** Boolean expression to compare columns of type "Boolean". All fields are combined with logical 'AND'. */
export type Boolean_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['Boolean']>;
  _gt?: InputMaybe<Scalars['Boolean']>;
  _gte?: InputMaybe<Scalars['Boolean']>;
  _in?: InputMaybe<Array<Scalars['Boolean']>>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  _lt?: InputMaybe<Scalars['Boolean']>;
  _lte?: InputMaybe<Scalars['Boolean']>;
  _neq?: InputMaybe<Scalars['Boolean']>;
  _nin?: InputMaybe<Array<Scalars['Boolean']>>;
};

/** Boolean expression to compare columns of type "Int". All fields are combined with logical 'AND'. */
export type Int_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['Int']>;
  _gt?: InputMaybe<Scalars['Int']>;
  _gte?: InputMaybe<Scalars['Int']>;
  _in?: InputMaybe<Array<Scalars['Int']>>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  _lt?: InputMaybe<Scalars['Int']>;
  _lte?: InputMaybe<Scalars['Int']>;
  _neq?: InputMaybe<Scalars['Int']>;
  _nin?: InputMaybe<Array<Scalars['Int']>>;
};

/** Boolean expression to compare columns of type "String". All fields are combined with logical 'AND'. */
export type String_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['String']>;
  _gt?: InputMaybe<Scalars['String']>;
  _gte?: InputMaybe<Scalars['String']>;
  /** does the column match the given case-insensitive pattern */
  _ilike?: InputMaybe<Scalars['String']>;
  _in?: InputMaybe<Array<Scalars['String']>>;
  /** does the column match the given POSIX regular expression, case insensitive */
  _iregex?: InputMaybe<Scalars['String']>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  /** does the column match the given pattern */
  _like?: InputMaybe<Scalars['String']>;
  _lt?: InputMaybe<Scalars['String']>;
  _lte?: InputMaybe<Scalars['String']>;
  _neq?: InputMaybe<Scalars['String']>;
  /** does the column NOT match the given case-insensitive pattern */
  _nilike?: InputMaybe<Scalars['String']>;
  _nin?: InputMaybe<Array<Scalars['String']>>;
  /** does the column NOT match the given POSIX regular expression, case insensitive */
  _niregex?: InputMaybe<Scalars['String']>;
  /** does the column NOT match the given pattern */
  _nlike?: InputMaybe<Scalars['String']>;
  /** does the column NOT match the given POSIX regular expression, case sensitive */
  _nregex?: InputMaybe<Scalars['String']>;
  /** does the column NOT match the given SQL regular expression */
  _nsimilar?: InputMaybe<Scalars['String']>;
  /** does the column match the given POSIX regular expression, case sensitive */
  _regex?: InputMaybe<Scalars['String']>;
  /** does the column match the given SQL regular expression */
  _similar?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "account_transactions" */
export type Account_Transactions = {
  __typename?: 'account_transactions';
  account_address: Scalars['String'];
  /** An array relationship */
  coin_activities: Array<Coin_Activities>;
  /** An aggregate relationship */
  coin_activities_aggregate: Coin_Activities_Aggregate;
  /** An array relationship */
  delegated_staking_activities: Array<Delegated_Staking_Activities>;
  /** An array relationship */
  fungible_asset_activities: Array<Fungible_Asset_Activities>;
  /** An array relationship */
  token_activities: Array<Token_Activities>;
  /** An aggregate relationship */
  token_activities_aggregate: Token_Activities_Aggregate;
  /** An array relationship */
  token_activities_v2: Array<Token_Activities_V2>;
  /** An aggregate relationship */
  token_activities_v2_aggregate: Token_Activities_V2_Aggregate;
  transaction_version: Scalars['bigint'];
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsCoin_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsCoin_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsDelegated_Staking_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Delegated_Staking_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegated_Staking_Activities_Order_By>>;
  where?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsFungible_Asset_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Fungible_Asset_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Fungible_Asset_Activities_Order_By>>;
  where?: InputMaybe<Fungible_Asset_Activities_Bool_Exp>;
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsToken_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsToken_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsToken_Activities_V2Args = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


/** columns and relationships of "account_transactions" */
export type Account_TransactionsToken_Activities_V2_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};

/** aggregated selection of "account_transactions" */
export type Account_Transactions_Aggregate = {
  __typename?: 'account_transactions_aggregate';
  aggregate?: Maybe<Account_Transactions_Aggregate_Fields>;
  nodes: Array<Account_Transactions>;
};

/** aggregate fields of "account_transactions" */
export type Account_Transactions_Aggregate_Fields = {
  __typename?: 'account_transactions_aggregate_fields';
  avg?: Maybe<Account_Transactions_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Account_Transactions_Max_Fields>;
  min?: Maybe<Account_Transactions_Min_Fields>;
  stddev?: Maybe<Account_Transactions_Stddev_Fields>;
  stddev_pop?: Maybe<Account_Transactions_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Account_Transactions_Stddev_Samp_Fields>;
  sum?: Maybe<Account_Transactions_Sum_Fields>;
  var_pop?: Maybe<Account_Transactions_Var_Pop_Fields>;
  var_samp?: Maybe<Account_Transactions_Var_Samp_Fields>;
  variance?: Maybe<Account_Transactions_Variance_Fields>;
};


/** aggregate fields of "account_transactions" */
export type Account_Transactions_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Account_Transactions_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** aggregate avg on columns */
export type Account_Transactions_Avg_Fields = {
  __typename?: 'account_transactions_avg_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to filter rows from the table "account_transactions". All fields are combined with a logical 'AND'. */
export type Account_Transactions_Bool_Exp = {
  _and?: InputMaybe<Array<Account_Transactions_Bool_Exp>>;
  _not?: InputMaybe<Account_Transactions_Bool_Exp>;
  _or?: InputMaybe<Array<Account_Transactions_Bool_Exp>>;
  account_address?: InputMaybe<String_Comparison_Exp>;
  coin_activities?: InputMaybe<Coin_Activities_Bool_Exp>;
  delegated_staking_activities?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
  fungible_asset_activities?: InputMaybe<Fungible_Asset_Activities_Bool_Exp>;
  token_activities?: InputMaybe<Token_Activities_Bool_Exp>;
  token_activities_v2?: InputMaybe<Token_Activities_V2_Bool_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** aggregate max on columns */
export type Account_Transactions_Max_Fields = {
  __typename?: 'account_transactions_max_fields';
  account_address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate min on columns */
export type Account_Transactions_Min_Fields = {
  __typename?: 'account_transactions_min_fields';
  account_address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** Ordering options when selecting data from "account_transactions". */
export type Account_Transactions_Order_By = {
  account_address?: InputMaybe<Order_By>;
  coin_activities_aggregate?: InputMaybe<Coin_Activities_Aggregate_Order_By>;
  delegated_staking_activities_aggregate?: InputMaybe<Delegated_Staking_Activities_Aggregate_Order_By>;
  fungible_asset_activities_aggregate?: InputMaybe<Fungible_Asset_Activities_Aggregate_Order_By>;
  token_activities_aggregate?: InputMaybe<Token_Activities_Aggregate_Order_By>;
  token_activities_v2_aggregate?: InputMaybe<Token_Activities_V2_Aggregate_Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "account_transactions" */
export enum Account_Transactions_Select_Column {
  /** column name */
  AccountAddress = 'account_address',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** aggregate stddev on columns */
export type Account_Transactions_Stddev_Fields = {
  __typename?: 'account_transactions_stddev_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_pop on columns */
export type Account_Transactions_Stddev_Pop_Fields = {
  __typename?: 'account_transactions_stddev_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_samp on columns */
export type Account_Transactions_Stddev_Samp_Fields = {
  __typename?: 'account_transactions_stddev_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Streaming cursor of the table "account_transactions" */
export type Account_Transactions_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Account_Transactions_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Account_Transactions_Stream_Cursor_Value_Input = {
  account_address?: InputMaybe<Scalars['String']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** aggregate sum on columns */
export type Account_Transactions_Sum_Fields = {
  __typename?: 'account_transactions_sum_fields';
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate var_pop on columns */
export type Account_Transactions_Var_Pop_Fields = {
  __typename?: 'account_transactions_var_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate var_samp on columns */
export type Account_Transactions_Var_Samp_Fields = {
  __typename?: 'account_transactions_var_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate variance on columns */
export type Account_Transactions_Variance_Fields = {
  __typename?: 'account_transactions_variance_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** columns and relationships of "address_events_summary" */
export type Address_Events_Summary = {
  __typename?: 'address_events_summary';
  account_address?: Maybe<Scalars['String']>;
  /** An object relationship */
  block_metadata?: Maybe<Block_Metadata_Transactions>;
  min_block_height?: Maybe<Scalars['bigint']>;
  num_distinct_versions?: Maybe<Scalars['bigint']>;
};

/** Boolean expression to filter rows from the table "address_events_summary". All fields are combined with a logical 'AND'. */
export type Address_Events_Summary_Bool_Exp = {
  _and?: InputMaybe<Array<Address_Events_Summary_Bool_Exp>>;
  _not?: InputMaybe<Address_Events_Summary_Bool_Exp>;
  _or?: InputMaybe<Array<Address_Events_Summary_Bool_Exp>>;
  account_address?: InputMaybe<String_Comparison_Exp>;
  block_metadata?: InputMaybe<Block_Metadata_Transactions_Bool_Exp>;
  min_block_height?: InputMaybe<Bigint_Comparison_Exp>;
  num_distinct_versions?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "address_events_summary". */
export type Address_Events_Summary_Order_By = {
  account_address?: InputMaybe<Order_By>;
  block_metadata?: InputMaybe<Block_Metadata_Transactions_Order_By>;
  min_block_height?: InputMaybe<Order_By>;
  num_distinct_versions?: InputMaybe<Order_By>;
};

/** select columns of table "address_events_summary" */
export enum Address_Events_Summary_Select_Column {
  /** column name */
  AccountAddress = 'account_address',
  /** column name */
  MinBlockHeight = 'min_block_height',
  /** column name */
  NumDistinctVersions = 'num_distinct_versions'
}

/** Streaming cursor of the table "address_events_summary" */
export type Address_Events_Summary_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Address_Events_Summary_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Address_Events_Summary_Stream_Cursor_Value_Input = {
  account_address?: InputMaybe<Scalars['String']>;
  min_block_height?: InputMaybe<Scalars['bigint']>;
  num_distinct_versions?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_Events = {
  __typename?: 'address_version_from_events';
  account_address?: Maybe<Scalars['String']>;
  /** An array relationship */
  coin_activities: Array<Coin_Activities>;
  /** An aggregate relationship */
  coin_activities_aggregate: Coin_Activities_Aggregate;
  /** An array relationship */
  delegated_staking_activities: Array<Delegated_Staking_Activities>;
  /** An array relationship */
  token_activities: Array<Token_Activities>;
  /** An aggregate relationship */
  token_activities_aggregate: Token_Activities_Aggregate;
  /** An array relationship */
  token_activities_v2: Array<Token_Activities_V2>;
  /** An aggregate relationship */
  token_activities_v2_aggregate: Token_Activities_V2_Aggregate;
  transaction_version?: Maybe<Scalars['bigint']>;
};


/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_EventsCoin_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_EventsCoin_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_EventsDelegated_Staking_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Delegated_Staking_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegated_Staking_Activities_Order_By>>;
  where?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_EventsToken_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_EventsToken_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_EventsToken_Activities_V2Args = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


/** columns and relationships of "address_version_from_events" */
export type Address_Version_From_EventsToken_Activities_V2_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};

/** aggregated selection of "address_version_from_events" */
export type Address_Version_From_Events_Aggregate = {
  __typename?: 'address_version_from_events_aggregate';
  aggregate?: Maybe<Address_Version_From_Events_Aggregate_Fields>;
  nodes: Array<Address_Version_From_Events>;
};

/** aggregate fields of "address_version_from_events" */
export type Address_Version_From_Events_Aggregate_Fields = {
  __typename?: 'address_version_from_events_aggregate_fields';
  avg?: Maybe<Address_Version_From_Events_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Address_Version_From_Events_Max_Fields>;
  min?: Maybe<Address_Version_From_Events_Min_Fields>;
  stddev?: Maybe<Address_Version_From_Events_Stddev_Fields>;
  stddev_pop?: Maybe<Address_Version_From_Events_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Address_Version_From_Events_Stddev_Samp_Fields>;
  sum?: Maybe<Address_Version_From_Events_Sum_Fields>;
  var_pop?: Maybe<Address_Version_From_Events_Var_Pop_Fields>;
  var_samp?: Maybe<Address_Version_From_Events_Var_Samp_Fields>;
  variance?: Maybe<Address_Version_From_Events_Variance_Fields>;
};


/** aggregate fields of "address_version_from_events" */
export type Address_Version_From_Events_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Address_Version_From_Events_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** aggregate avg on columns */
export type Address_Version_From_Events_Avg_Fields = {
  __typename?: 'address_version_from_events_avg_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to filter rows from the table "address_version_from_events". All fields are combined with a logical 'AND'. */
export type Address_Version_From_Events_Bool_Exp = {
  _and?: InputMaybe<Array<Address_Version_From_Events_Bool_Exp>>;
  _not?: InputMaybe<Address_Version_From_Events_Bool_Exp>;
  _or?: InputMaybe<Array<Address_Version_From_Events_Bool_Exp>>;
  account_address?: InputMaybe<String_Comparison_Exp>;
  coin_activities?: InputMaybe<Coin_Activities_Bool_Exp>;
  delegated_staking_activities?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
  token_activities?: InputMaybe<Token_Activities_Bool_Exp>;
  token_activities_v2?: InputMaybe<Token_Activities_V2_Bool_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** aggregate max on columns */
export type Address_Version_From_Events_Max_Fields = {
  __typename?: 'address_version_from_events_max_fields';
  account_address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate min on columns */
export type Address_Version_From_Events_Min_Fields = {
  __typename?: 'address_version_from_events_min_fields';
  account_address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** Ordering options when selecting data from "address_version_from_events". */
export type Address_Version_From_Events_Order_By = {
  account_address?: InputMaybe<Order_By>;
  coin_activities_aggregate?: InputMaybe<Coin_Activities_Aggregate_Order_By>;
  delegated_staking_activities_aggregate?: InputMaybe<Delegated_Staking_Activities_Aggregate_Order_By>;
  token_activities_aggregate?: InputMaybe<Token_Activities_Aggregate_Order_By>;
  token_activities_v2_aggregate?: InputMaybe<Token_Activities_V2_Aggregate_Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "address_version_from_events" */
export enum Address_Version_From_Events_Select_Column {
  /** column name */
  AccountAddress = 'account_address',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** aggregate stddev on columns */
export type Address_Version_From_Events_Stddev_Fields = {
  __typename?: 'address_version_from_events_stddev_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_pop on columns */
export type Address_Version_From_Events_Stddev_Pop_Fields = {
  __typename?: 'address_version_from_events_stddev_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_samp on columns */
export type Address_Version_From_Events_Stddev_Samp_Fields = {
  __typename?: 'address_version_from_events_stddev_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Streaming cursor of the table "address_version_from_events" */
export type Address_Version_From_Events_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Address_Version_From_Events_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Address_Version_From_Events_Stream_Cursor_Value_Input = {
  account_address?: InputMaybe<Scalars['String']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** aggregate sum on columns */
export type Address_Version_From_Events_Sum_Fields = {
  __typename?: 'address_version_from_events_sum_fields';
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate var_pop on columns */
export type Address_Version_From_Events_Var_Pop_Fields = {
  __typename?: 'address_version_from_events_var_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate var_samp on columns */
export type Address_Version_From_Events_Var_Samp_Fields = {
  __typename?: 'address_version_from_events_var_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate variance on columns */
export type Address_Version_From_Events_Variance_Fields = {
  __typename?: 'address_version_from_events_variance_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_Resources = {
  __typename?: 'address_version_from_move_resources';
  address?: Maybe<Scalars['String']>;
  /** An array relationship */
  coin_activities: Array<Coin_Activities>;
  /** An aggregate relationship */
  coin_activities_aggregate: Coin_Activities_Aggregate;
  /** An array relationship */
  delegated_staking_activities: Array<Delegated_Staking_Activities>;
  /** An array relationship */
  token_activities: Array<Token_Activities>;
  /** An aggregate relationship */
  token_activities_aggregate: Token_Activities_Aggregate;
  /** An array relationship */
  token_activities_v2: Array<Token_Activities_V2>;
  /** An aggregate relationship */
  token_activities_v2_aggregate: Token_Activities_V2_Aggregate;
  transaction_version?: Maybe<Scalars['bigint']>;
};


/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_ResourcesCoin_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_ResourcesCoin_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_ResourcesDelegated_Staking_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Delegated_Staking_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegated_Staking_Activities_Order_By>>;
  where?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_ResourcesToken_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_ResourcesToken_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_ResourcesToken_Activities_V2Args = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


/** columns and relationships of "address_version_from_move_resources" */
export type Address_Version_From_Move_ResourcesToken_Activities_V2_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};

/** aggregated selection of "address_version_from_move_resources" */
export type Address_Version_From_Move_Resources_Aggregate = {
  __typename?: 'address_version_from_move_resources_aggregate';
  aggregate?: Maybe<Address_Version_From_Move_Resources_Aggregate_Fields>;
  nodes: Array<Address_Version_From_Move_Resources>;
};

/** aggregate fields of "address_version_from_move_resources" */
export type Address_Version_From_Move_Resources_Aggregate_Fields = {
  __typename?: 'address_version_from_move_resources_aggregate_fields';
  avg?: Maybe<Address_Version_From_Move_Resources_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Address_Version_From_Move_Resources_Max_Fields>;
  min?: Maybe<Address_Version_From_Move_Resources_Min_Fields>;
  stddev?: Maybe<Address_Version_From_Move_Resources_Stddev_Fields>;
  stddev_pop?: Maybe<Address_Version_From_Move_Resources_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Address_Version_From_Move_Resources_Stddev_Samp_Fields>;
  sum?: Maybe<Address_Version_From_Move_Resources_Sum_Fields>;
  var_pop?: Maybe<Address_Version_From_Move_Resources_Var_Pop_Fields>;
  var_samp?: Maybe<Address_Version_From_Move_Resources_Var_Samp_Fields>;
  variance?: Maybe<Address_Version_From_Move_Resources_Variance_Fields>;
};


/** aggregate fields of "address_version_from_move_resources" */
export type Address_Version_From_Move_Resources_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Address_Version_From_Move_Resources_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** aggregate avg on columns */
export type Address_Version_From_Move_Resources_Avg_Fields = {
  __typename?: 'address_version_from_move_resources_avg_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to filter rows from the table "address_version_from_move_resources". All fields are combined with a logical 'AND'. */
export type Address_Version_From_Move_Resources_Bool_Exp = {
  _and?: InputMaybe<Array<Address_Version_From_Move_Resources_Bool_Exp>>;
  _not?: InputMaybe<Address_Version_From_Move_Resources_Bool_Exp>;
  _or?: InputMaybe<Array<Address_Version_From_Move_Resources_Bool_Exp>>;
  address?: InputMaybe<String_Comparison_Exp>;
  coin_activities?: InputMaybe<Coin_Activities_Bool_Exp>;
  delegated_staking_activities?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
  token_activities?: InputMaybe<Token_Activities_Bool_Exp>;
  token_activities_v2?: InputMaybe<Token_Activities_V2_Bool_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** aggregate max on columns */
export type Address_Version_From_Move_Resources_Max_Fields = {
  __typename?: 'address_version_from_move_resources_max_fields';
  address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate min on columns */
export type Address_Version_From_Move_Resources_Min_Fields = {
  __typename?: 'address_version_from_move_resources_min_fields';
  address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** Ordering options when selecting data from "address_version_from_move_resources". */
export type Address_Version_From_Move_Resources_Order_By = {
  address?: InputMaybe<Order_By>;
  coin_activities_aggregate?: InputMaybe<Coin_Activities_Aggregate_Order_By>;
  delegated_staking_activities_aggregate?: InputMaybe<Delegated_Staking_Activities_Aggregate_Order_By>;
  token_activities_aggregate?: InputMaybe<Token_Activities_Aggregate_Order_By>;
  token_activities_v2_aggregate?: InputMaybe<Token_Activities_V2_Aggregate_Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "address_version_from_move_resources" */
export enum Address_Version_From_Move_Resources_Select_Column {
  /** column name */
  Address = 'address',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** aggregate stddev on columns */
export type Address_Version_From_Move_Resources_Stddev_Fields = {
  __typename?: 'address_version_from_move_resources_stddev_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_pop on columns */
export type Address_Version_From_Move_Resources_Stddev_Pop_Fields = {
  __typename?: 'address_version_from_move_resources_stddev_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_samp on columns */
export type Address_Version_From_Move_Resources_Stddev_Samp_Fields = {
  __typename?: 'address_version_from_move_resources_stddev_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Streaming cursor of the table "address_version_from_move_resources" */
export type Address_Version_From_Move_Resources_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Address_Version_From_Move_Resources_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Address_Version_From_Move_Resources_Stream_Cursor_Value_Input = {
  address?: InputMaybe<Scalars['String']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** aggregate sum on columns */
export type Address_Version_From_Move_Resources_Sum_Fields = {
  __typename?: 'address_version_from_move_resources_sum_fields';
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate var_pop on columns */
export type Address_Version_From_Move_Resources_Var_Pop_Fields = {
  __typename?: 'address_version_from_move_resources_var_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate var_samp on columns */
export type Address_Version_From_Move_Resources_Var_Samp_Fields = {
  __typename?: 'address_version_from_move_resources_var_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate variance on columns */
export type Address_Version_From_Move_Resources_Variance_Fields = {
  __typename?: 'address_version_from_move_resources_variance_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to compare columns of type "bigint". All fields are combined with logical 'AND'. */
export type Bigint_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['bigint']>;
  _gt?: InputMaybe<Scalars['bigint']>;
  _gte?: InputMaybe<Scalars['bigint']>;
  _in?: InputMaybe<Array<Scalars['bigint']>>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  _lt?: InputMaybe<Scalars['bigint']>;
  _lte?: InputMaybe<Scalars['bigint']>;
  _neq?: InputMaybe<Scalars['bigint']>;
  _nin?: InputMaybe<Array<Scalars['bigint']>>;
};

/** columns and relationships of "block_metadata_transactions" */
export type Block_Metadata_Transactions = {
  __typename?: 'block_metadata_transactions';
  block_height: Scalars['bigint'];
  epoch: Scalars['bigint'];
  failed_proposer_indices: Scalars['jsonb'];
  id: Scalars['String'];
  previous_block_votes_bitvec: Scalars['jsonb'];
  proposer: Scalars['String'];
  round: Scalars['bigint'];
  timestamp: Scalars['timestamp'];
  version: Scalars['bigint'];
};


/** columns and relationships of "block_metadata_transactions" */
export type Block_Metadata_TransactionsFailed_Proposer_IndicesArgs = {
  path?: InputMaybe<Scalars['String']>;
};


/** columns and relationships of "block_metadata_transactions" */
export type Block_Metadata_TransactionsPrevious_Block_Votes_BitvecArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "block_metadata_transactions". All fields are combined with a logical 'AND'. */
export type Block_Metadata_Transactions_Bool_Exp = {
  _and?: InputMaybe<Array<Block_Metadata_Transactions_Bool_Exp>>;
  _not?: InputMaybe<Block_Metadata_Transactions_Bool_Exp>;
  _or?: InputMaybe<Array<Block_Metadata_Transactions_Bool_Exp>>;
  block_height?: InputMaybe<Bigint_Comparison_Exp>;
  epoch?: InputMaybe<Bigint_Comparison_Exp>;
  failed_proposer_indices?: InputMaybe<Jsonb_Comparison_Exp>;
  id?: InputMaybe<String_Comparison_Exp>;
  previous_block_votes_bitvec?: InputMaybe<Jsonb_Comparison_Exp>;
  proposer?: InputMaybe<String_Comparison_Exp>;
  round?: InputMaybe<Bigint_Comparison_Exp>;
  timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "block_metadata_transactions". */
export type Block_Metadata_Transactions_Order_By = {
  block_height?: InputMaybe<Order_By>;
  epoch?: InputMaybe<Order_By>;
  failed_proposer_indices?: InputMaybe<Order_By>;
  id?: InputMaybe<Order_By>;
  previous_block_votes_bitvec?: InputMaybe<Order_By>;
  proposer?: InputMaybe<Order_By>;
  round?: InputMaybe<Order_By>;
  timestamp?: InputMaybe<Order_By>;
  version?: InputMaybe<Order_By>;
};

/** select columns of table "block_metadata_transactions" */
export enum Block_Metadata_Transactions_Select_Column {
  /** column name */
  BlockHeight = 'block_height',
  /** column name */
  Epoch = 'epoch',
  /** column name */
  FailedProposerIndices = 'failed_proposer_indices',
  /** column name */
  Id = 'id',
  /** column name */
  PreviousBlockVotesBitvec = 'previous_block_votes_bitvec',
  /** column name */
  Proposer = 'proposer',
  /** column name */
  Round = 'round',
  /** column name */
  Timestamp = 'timestamp',
  /** column name */
  Version = 'version'
}

/** Streaming cursor of the table "block_metadata_transactions" */
export type Block_Metadata_Transactions_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Block_Metadata_Transactions_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Block_Metadata_Transactions_Stream_Cursor_Value_Input = {
  block_height?: InputMaybe<Scalars['bigint']>;
  epoch?: InputMaybe<Scalars['bigint']>;
  failed_proposer_indices?: InputMaybe<Scalars['jsonb']>;
  id?: InputMaybe<Scalars['String']>;
  previous_block_votes_bitvec?: InputMaybe<Scalars['jsonb']>;
  proposer?: InputMaybe<Scalars['String']>;
  round?: InputMaybe<Scalars['bigint']>;
  timestamp?: InputMaybe<Scalars['timestamp']>;
  version?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "coin_activities" */
export type Coin_Activities = {
  __typename?: 'coin_activities';
  activity_type: Scalars['String'];
  amount: Scalars['numeric'];
  /** An array relationship */
  aptos_names: Array<Current_Aptos_Names>;
  block_height: Scalars['bigint'];
  /** An object relationship */
  coin_info?: Maybe<Coin_Infos>;
  coin_type: Scalars['String'];
  entry_function_id_str?: Maybe<Scalars['String']>;
  event_account_address: Scalars['String'];
  event_creation_number: Scalars['bigint'];
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number: Scalars['bigint'];
  is_gas_fee: Scalars['Boolean'];
  is_transaction_success: Scalars['Boolean'];
  owner_address: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
};


/** columns and relationships of "coin_activities" */
export type Coin_ActivitiesAptos_NamesArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};

/** aggregated selection of "coin_activities" */
export type Coin_Activities_Aggregate = {
  __typename?: 'coin_activities_aggregate';
  aggregate?: Maybe<Coin_Activities_Aggregate_Fields>;
  nodes: Array<Coin_Activities>;
};

/** aggregate fields of "coin_activities" */
export type Coin_Activities_Aggregate_Fields = {
  __typename?: 'coin_activities_aggregate_fields';
  avg?: Maybe<Coin_Activities_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Coin_Activities_Max_Fields>;
  min?: Maybe<Coin_Activities_Min_Fields>;
  stddev?: Maybe<Coin_Activities_Stddev_Fields>;
  stddev_pop?: Maybe<Coin_Activities_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Coin_Activities_Stddev_Samp_Fields>;
  sum?: Maybe<Coin_Activities_Sum_Fields>;
  var_pop?: Maybe<Coin_Activities_Var_Pop_Fields>;
  var_samp?: Maybe<Coin_Activities_Var_Samp_Fields>;
  variance?: Maybe<Coin_Activities_Variance_Fields>;
};


/** aggregate fields of "coin_activities" */
export type Coin_Activities_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** order by aggregate values of table "coin_activities" */
export type Coin_Activities_Aggregate_Order_By = {
  avg?: InputMaybe<Coin_Activities_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Coin_Activities_Max_Order_By>;
  min?: InputMaybe<Coin_Activities_Min_Order_By>;
  stddev?: InputMaybe<Coin_Activities_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Coin_Activities_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Coin_Activities_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Coin_Activities_Sum_Order_By>;
  var_pop?: InputMaybe<Coin_Activities_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Coin_Activities_Var_Samp_Order_By>;
  variance?: InputMaybe<Coin_Activities_Variance_Order_By>;
};

/** aggregate avg on columns */
export type Coin_Activities_Avg_Fields = {
  __typename?: 'coin_activities_avg_fields';
  amount?: Maybe<Scalars['Float']>;
  block_height?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by avg() on columns of table "coin_activities" */
export type Coin_Activities_Avg_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "coin_activities". All fields are combined with a logical 'AND'. */
export type Coin_Activities_Bool_Exp = {
  _and?: InputMaybe<Array<Coin_Activities_Bool_Exp>>;
  _not?: InputMaybe<Coin_Activities_Bool_Exp>;
  _or?: InputMaybe<Array<Coin_Activities_Bool_Exp>>;
  activity_type?: InputMaybe<String_Comparison_Exp>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  aptos_names?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  block_height?: InputMaybe<Bigint_Comparison_Exp>;
  coin_info?: InputMaybe<Coin_Infos_Bool_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  event_account_address?: InputMaybe<String_Comparison_Exp>;
  event_creation_number?: InputMaybe<Bigint_Comparison_Exp>;
  event_index?: InputMaybe<Bigint_Comparison_Exp>;
  event_sequence_number?: InputMaybe<Bigint_Comparison_Exp>;
  is_gas_fee?: InputMaybe<Boolean_Comparison_Exp>;
  is_transaction_success?: InputMaybe<Boolean_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** aggregate max on columns */
export type Coin_Activities_Max_Fields = {
  __typename?: 'coin_activities_max_fields';
  activity_type?: Maybe<Scalars['String']>;
  amount?: Maybe<Scalars['numeric']>;
  block_height?: Maybe<Scalars['bigint']>;
  coin_type?: Maybe<Scalars['String']>;
  entry_function_id_str?: Maybe<Scalars['String']>;
  event_account_address?: Maybe<Scalars['String']>;
  event_creation_number?: Maybe<Scalars['bigint']>;
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** order by max() on columns of table "coin_activities" */
export type Coin_Activities_Max_Order_By = {
  activity_type?: InputMaybe<Order_By>;
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Coin_Activities_Min_Fields = {
  __typename?: 'coin_activities_min_fields';
  activity_type?: Maybe<Scalars['String']>;
  amount?: Maybe<Scalars['numeric']>;
  block_height?: Maybe<Scalars['bigint']>;
  coin_type?: Maybe<Scalars['String']>;
  entry_function_id_str?: Maybe<Scalars['String']>;
  event_account_address?: Maybe<Scalars['String']>;
  event_creation_number?: Maybe<Scalars['bigint']>;
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** order by min() on columns of table "coin_activities" */
export type Coin_Activities_Min_Order_By = {
  activity_type?: InputMaybe<Order_By>;
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "coin_activities". */
export type Coin_Activities_Order_By = {
  activity_type?: InputMaybe<Order_By>;
  amount?: InputMaybe<Order_By>;
  aptos_names_aggregate?: InputMaybe<Current_Aptos_Names_Aggregate_Order_By>;
  block_height?: InputMaybe<Order_By>;
  coin_info?: InputMaybe<Coin_Infos_Order_By>;
  coin_type?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  is_gas_fee?: InputMaybe<Order_By>;
  is_transaction_success?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "coin_activities" */
export enum Coin_Activities_Select_Column {
  /** column name */
  ActivityType = 'activity_type',
  /** column name */
  Amount = 'amount',
  /** column name */
  BlockHeight = 'block_height',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  EventAccountAddress = 'event_account_address',
  /** column name */
  EventCreationNumber = 'event_creation_number',
  /** column name */
  EventIndex = 'event_index',
  /** column name */
  EventSequenceNumber = 'event_sequence_number',
  /** column name */
  IsGasFee = 'is_gas_fee',
  /** column name */
  IsTransactionSuccess = 'is_transaction_success',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** aggregate stddev on columns */
export type Coin_Activities_Stddev_Fields = {
  __typename?: 'coin_activities_stddev_fields';
  amount?: Maybe<Scalars['Float']>;
  block_height?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev() on columns of table "coin_activities" */
export type Coin_Activities_Stddev_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_pop on columns */
export type Coin_Activities_Stddev_Pop_Fields = {
  __typename?: 'coin_activities_stddev_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  block_height?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_pop() on columns of table "coin_activities" */
export type Coin_Activities_Stddev_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_samp on columns */
export type Coin_Activities_Stddev_Samp_Fields = {
  __typename?: 'coin_activities_stddev_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  block_height?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_samp() on columns of table "coin_activities" */
export type Coin_Activities_Stddev_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "coin_activities" */
export type Coin_Activities_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Coin_Activities_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Coin_Activities_Stream_Cursor_Value_Input = {
  activity_type?: InputMaybe<Scalars['String']>;
  amount?: InputMaybe<Scalars['numeric']>;
  block_height?: InputMaybe<Scalars['bigint']>;
  coin_type?: InputMaybe<Scalars['String']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  event_account_address?: InputMaybe<Scalars['String']>;
  event_creation_number?: InputMaybe<Scalars['bigint']>;
  event_index?: InputMaybe<Scalars['bigint']>;
  event_sequence_number?: InputMaybe<Scalars['bigint']>;
  is_gas_fee?: InputMaybe<Scalars['Boolean']>;
  is_transaction_success?: InputMaybe<Scalars['Boolean']>;
  owner_address?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** aggregate sum on columns */
export type Coin_Activities_Sum_Fields = {
  __typename?: 'coin_activities_sum_fields';
  amount?: Maybe<Scalars['numeric']>;
  block_height?: Maybe<Scalars['bigint']>;
  event_creation_number?: Maybe<Scalars['bigint']>;
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number?: Maybe<Scalars['bigint']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** order by sum() on columns of table "coin_activities" */
export type Coin_Activities_Sum_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate var_pop on columns */
export type Coin_Activities_Var_Pop_Fields = {
  __typename?: 'coin_activities_var_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  block_height?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by var_pop() on columns of table "coin_activities" */
export type Coin_Activities_Var_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate var_samp on columns */
export type Coin_Activities_Var_Samp_Fields = {
  __typename?: 'coin_activities_var_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  block_height?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by var_samp() on columns of table "coin_activities" */
export type Coin_Activities_Var_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate variance on columns */
export type Coin_Activities_Variance_Fields = {
  __typename?: 'coin_activities_variance_fields';
  amount?: Maybe<Scalars['Float']>;
  block_height?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by variance() on columns of table "coin_activities" */
export type Coin_Activities_Variance_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "coin_balances" */
export type Coin_Balances = {
  __typename?: 'coin_balances';
  amount: Scalars['numeric'];
  coin_type: Scalars['String'];
  coin_type_hash: Scalars['String'];
  owner_address: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
};

/** Boolean expression to filter rows from the table "coin_balances". All fields are combined with a logical 'AND'. */
export type Coin_Balances_Bool_Exp = {
  _and?: InputMaybe<Array<Coin_Balances_Bool_Exp>>;
  _not?: InputMaybe<Coin_Balances_Bool_Exp>;
  _or?: InputMaybe<Array<Coin_Balances_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  coin_type_hash?: InputMaybe<String_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "coin_balances". */
export type Coin_Balances_Order_By = {
  amount?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  coin_type_hash?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "coin_balances" */
export enum Coin_Balances_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CoinTypeHash = 'coin_type_hash',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** Streaming cursor of the table "coin_balances" */
export type Coin_Balances_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Coin_Balances_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Coin_Balances_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  coin_type?: InputMaybe<Scalars['String']>;
  coin_type_hash?: InputMaybe<Scalars['String']>;
  owner_address?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "coin_infos" */
export type Coin_Infos = {
  __typename?: 'coin_infos';
  coin_type: Scalars['String'];
  coin_type_hash: Scalars['String'];
  creator_address: Scalars['String'];
  decimals: Scalars['Int'];
  name: Scalars['String'];
  supply_aggregator_table_handle?: Maybe<Scalars['String']>;
  supply_aggregator_table_key?: Maybe<Scalars['String']>;
  symbol: Scalars['String'];
  transaction_created_timestamp: Scalars['timestamp'];
  transaction_version_created: Scalars['bigint'];
};

/** Boolean expression to filter rows from the table "coin_infos". All fields are combined with a logical 'AND'. */
export type Coin_Infos_Bool_Exp = {
  _and?: InputMaybe<Array<Coin_Infos_Bool_Exp>>;
  _not?: InputMaybe<Coin_Infos_Bool_Exp>;
  _or?: InputMaybe<Array<Coin_Infos_Bool_Exp>>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  coin_type_hash?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  decimals?: InputMaybe<Int_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  supply_aggregator_table_handle?: InputMaybe<String_Comparison_Exp>;
  supply_aggregator_table_key?: InputMaybe<String_Comparison_Exp>;
  symbol?: InputMaybe<String_Comparison_Exp>;
  transaction_created_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version_created?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "coin_infos". */
export type Coin_Infos_Order_By = {
  coin_type?: InputMaybe<Order_By>;
  coin_type_hash?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  decimals?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  supply_aggregator_table_handle?: InputMaybe<Order_By>;
  supply_aggregator_table_key?: InputMaybe<Order_By>;
  symbol?: InputMaybe<Order_By>;
  transaction_created_timestamp?: InputMaybe<Order_By>;
  transaction_version_created?: InputMaybe<Order_By>;
};

/** select columns of table "coin_infos" */
export enum Coin_Infos_Select_Column {
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CoinTypeHash = 'coin_type_hash',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  Decimals = 'decimals',
  /** column name */
  Name = 'name',
  /** column name */
  SupplyAggregatorTableHandle = 'supply_aggregator_table_handle',
  /** column name */
  SupplyAggregatorTableKey = 'supply_aggregator_table_key',
  /** column name */
  Symbol = 'symbol',
  /** column name */
  TransactionCreatedTimestamp = 'transaction_created_timestamp',
  /** column name */
  TransactionVersionCreated = 'transaction_version_created'
}

/** Streaming cursor of the table "coin_infos" */
export type Coin_Infos_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Coin_Infos_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Coin_Infos_Stream_Cursor_Value_Input = {
  coin_type?: InputMaybe<Scalars['String']>;
  coin_type_hash?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  decimals?: InputMaybe<Scalars['Int']>;
  name?: InputMaybe<Scalars['String']>;
  supply_aggregator_table_handle?: InputMaybe<Scalars['String']>;
  supply_aggregator_table_key?: InputMaybe<Scalars['String']>;
  symbol?: InputMaybe<Scalars['String']>;
  transaction_created_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version_created?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "coin_supply" */
export type Coin_Supply = {
  __typename?: 'coin_supply';
  coin_type: Scalars['String'];
  coin_type_hash: Scalars['String'];
  supply: Scalars['numeric'];
  transaction_epoch: Scalars['bigint'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
};

/** Boolean expression to filter rows from the table "coin_supply". All fields are combined with a logical 'AND'. */
export type Coin_Supply_Bool_Exp = {
  _and?: InputMaybe<Array<Coin_Supply_Bool_Exp>>;
  _not?: InputMaybe<Coin_Supply_Bool_Exp>;
  _or?: InputMaybe<Array<Coin_Supply_Bool_Exp>>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  coin_type_hash?: InputMaybe<String_Comparison_Exp>;
  supply?: InputMaybe<Numeric_Comparison_Exp>;
  transaction_epoch?: InputMaybe<Bigint_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "coin_supply". */
export type Coin_Supply_Order_By = {
  coin_type?: InputMaybe<Order_By>;
  coin_type_hash?: InputMaybe<Order_By>;
  supply?: InputMaybe<Order_By>;
  transaction_epoch?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "coin_supply" */
export enum Coin_Supply_Select_Column {
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CoinTypeHash = 'coin_type_hash',
  /** column name */
  Supply = 'supply',
  /** column name */
  TransactionEpoch = 'transaction_epoch',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** Streaming cursor of the table "coin_supply" */
export type Coin_Supply_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Coin_Supply_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Coin_Supply_Stream_Cursor_Value_Input = {
  coin_type?: InputMaybe<Scalars['String']>;
  coin_type_hash?: InputMaybe<Scalars['String']>;
  supply?: InputMaybe<Scalars['numeric']>;
  transaction_epoch?: InputMaybe<Scalars['bigint']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "collection_datas" */
export type Collection_Datas = {
  __typename?: 'collection_datas';
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  description: Scalars['String'];
  description_mutable: Scalars['Boolean'];
  maximum: Scalars['numeric'];
  maximum_mutable: Scalars['Boolean'];
  metadata_uri: Scalars['String'];
  supply: Scalars['numeric'];
  table_handle: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
  uri_mutable: Scalars['Boolean'];
};

/** Boolean expression to filter rows from the table "collection_datas". All fields are combined with a logical 'AND'. */
export type Collection_Datas_Bool_Exp = {
  _and?: InputMaybe<Array<Collection_Datas_Bool_Exp>>;
  _not?: InputMaybe<Collection_Datas_Bool_Exp>;
  _or?: InputMaybe<Array<Collection_Datas_Bool_Exp>>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  description_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  maximum?: InputMaybe<Numeric_Comparison_Exp>;
  maximum_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  metadata_uri?: InputMaybe<String_Comparison_Exp>;
  supply?: InputMaybe<Numeric_Comparison_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  uri_mutable?: InputMaybe<Boolean_Comparison_Exp>;
};

/** Ordering options when selecting data from "collection_datas". */
export type Collection_Datas_Order_By = {
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  description_mutable?: InputMaybe<Order_By>;
  maximum?: InputMaybe<Order_By>;
  maximum_mutable?: InputMaybe<Order_By>;
  metadata_uri?: InputMaybe<Order_By>;
  supply?: InputMaybe<Order_By>;
  table_handle?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  uri_mutable?: InputMaybe<Order_By>;
};

/** select columns of table "collection_datas" */
export enum Collection_Datas_Select_Column {
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  Description = 'description',
  /** column name */
  DescriptionMutable = 'description_mutable',
  /** column name */
  Maximum = 'maximum',
  /** column name */
  MaximumMutable = 'maximum_mutable',
  /** column name */
  MetadataUri = 'metadata_uri',
  /** column name */
  Supply = 'supply',
  /** column name */
  TableHandle = 'table_handle',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  UriMutable = 'uri_mutable'
}

/** Streaming cursor of the table "collection_datas" */
export type Collection_Datas_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Collection_Datas_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Collection_Datas_Stream_Cursor_Value_Input = {
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  description?: InputMaybe<Scalars['String']>;
  description_mutable?: InputMaybe<Scalars['Boolean']>;
  maximum?: InputMaybe<Scalars['numeric']>;
  maximum_mutable?: InputMaybe<Scalars['Boolean']>;
  metadata_uri?: InputMaybe<Scalars['String']>;
  supply?: InputMaybe<Scalars['numeric']>;
  table_handle?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  uri_mutable?: InputMaybe<Scalars['Boolean']>;
};

/** columns and relationships of "current_ans_lookup" */
export type Current_Ans_Lookup = {
  __typename?: 'current_ans_lookup';
  /** An array relationship */
  all_token_ownerships: Array<Current_Token_Ownerships>;
  /** An aggregate relationship */
  all_token_ownerships_aggregate: Current_Token_Ownerships_Aggregate;
  domain: Scalars['String'];
  expiration_timestamp: Scalars['timestamp'];
  is_deleted: Scalars['Boolean'];
  last_transaction_version: Scalars['bigint'];
  registered_address?: Maybe<Scalars['String']>;
  subdomain: Scalars['String'];
  token_name: Scalars['String'];
};


/** columns and relationships of "current_ans_lookup" */
export type Current_Ans_LookupAll_Token_OwnershipsArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
};


/** columns and relationships of "current_ans_lookup" */
export type Current_Ans_LookupAll_Token_Ownerships_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
};

/** Boolean expression to filter rows from the table "current_ans_lookup". All fields are combined with a logical 'AND'. */
export type Current_Ans_Lookup_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Ans_Lookup_Bool_Exp>>;
  _not?: InputMaybe<Current_Ans_Lookup_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Ans_Lookup_Bool_Exp>>;
  all_token_ownerships?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
  domain?: InputMaybe<String_Comparison_Exp>;
  expiration_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  is_deleted?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  registered_address?: InputMaybe<String_Comparison_Exp>;
  subdomain?: InputMaybe<String_Comparison_Exp>;
  token_name?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_ans_lookup". */
export type Current_Ans_Lookup_Order_By = {
  all_token_ownerships_aggregate?: InputMaybe<Current_Token_Ownerships_Aggregate_Order_By>;
  domain?: InputMaybe<Order_By>;
  expiration_timestamp?: InputMaybe<Order_By>;
  is_deleted?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  registered_address?: InputMaybe<Order_By>;
  subdomain?: InputMaybe<Order_By>;
  token_name?: InputMaybe<Order_By>;
};

/** select columns of table "current_ans_lookup" */
export enum Current_Ans_Lookup_Select_Column {
  /** column name */
  Domain = 'domain',
  /** column name */
  ExpirationTimestamp = 'expiration_timestamp',
  /** column name */
  IsDeleted = 'is_deleted',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  RegisteredAddress = 'registered_address',
  /** column name */
  Subdomain = 'subdomain',
  /** column name */
  TokenName = 'token_name'
}

/** Streaming cursor of the table "current_ans_lookup" */
export type Current_Ans_Lookup_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Ans_Lookup_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Ans_Lookup_Stream_Cursor_Value_Input = {
  domain?: InputMaybe<Scalars['String']>;
  expiration_timestamp?: InputMaybe<Scalars['timestamp']>;
  is_deleted?: InputMaybe<Scalars['Boolean']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  registered_address?: InputMaybe<Scalars['String']>;
  subdomain?: InputMaybe<Scalars['String']>;
  token_name?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_aptos_names" */
export type Current_Aptos_Names = {
  __typename?: 'current_aptos_names';
  domain?: Maybe<Scalars['String']>;
  expiration_timestamp?: Maybe<Scalars['timestamp']>;
  is_primary?: Maybe<Scalars['Boolean']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  registered_address?: Maybe<Scalars['String']>;
  subdomain?: Maybe<Scalars['String']>;
  token_name?: Maybe<Scalars['String']>;
};

/** order by aggregate values of table "current_aptos_names" */
export type Current_Aptos_Names_Aggregate_Order_By = {
  avg?: InputMaybe<Current_Aptos_Names_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Current_Aptos_Names_Max_Order_By>;
  min?: InputMaybe<Current_Aptos_Names_Min_Order_By>;
  stddev?: InputMaybe<Current_Aptos_Names_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Current_Aptos_Names_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Current_Aptos_Names_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Current_Aptos_Names_Sum_Order_By>;
  var_pop?: InputMaybe<Current_Aptos_Names_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Current_Aptos_Names_Var_Samp_Order_By>;
  variance?: InputMaybe<Current_Aptos_Names_Variance_Order_By>;
};

/** order by avg() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Avg_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "current_aptos_names". All fields are combined with a logical 'AND'. */
export type Current_Aptos_Names_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Aptos_Names_Bool_Exp>>;
  _not?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Aptos_Names_Bool_Exp>>;
  domain?: InputMaybe<String_Comparison_Exp>;
  expiration_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  is_primary?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  registered_address?: InputMaybe<String_Comparison_Exp>;
  subdomain?: InputMaybe<String_Comparison_Exp>;
  token_name?: InputMaybe<String_Comparison_Exp>;
};

/** order by max() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Max_Order_By = {
  domain?: InputMaybe<Order_By>;
  expiration_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  registered_address?: InputMaybe<Order_By>;
  subdomain?: InputMaybe<Order_By>;
  token_name?: InputMaybe<Order_By>;
};

/** order by min() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Min_Order_By = {
  domain?: InputMaybe<Order_By>;
  expiration_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  registered_address?: InputMaybe<Order_By>;
  subdomain?: InputMaybe<Order_By>;
  token_name?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "current_aptos_names". */
export type Current_Aptos_Names_Order_By = {
  domain?: InputMaybe<Order_By>;
  expiration_timestamp?: InputMaybe<Order_By>;
  is_primary?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  registered_address?: InputMaybe<Order_By>;
  subdomain?: InputMaybe<Order_By>;
  token_name?: InputMaybe<Order_By>;
};

/** select columns of table "current_aptos_names" */
export enum Current_Aptos_Names_Select_Column {
  /** column name */
  Domain = 'domain',
  /** column name */
  ExpirationTimestamp = 'expiration_timestamp',
  /** column name */
  IsPrimary = 'is_primary',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  RegisteredAddress = 'registered_address',
  /** column name */
  Subdomain = 'subdomain',
  /** column name */
  TokenName = 'token_name'
}

/** order by stddev() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Stddev_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** order by stddev_pop() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Stddev_Pop_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** order by stddev_samp() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Stddev_Samp_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "current_aptos_names" */
export type Current_Aptos_Names_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Aptos_Names_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Aptos_Names_Stream_Cursor_Value_Input = {
  domain?: InputMaybe<Scalars['String']>;
  expiration_timestamp?: InputMaybe<Scalars['timestamp']>;
  is_primary?: InputMaybe<Scalars['Boolean']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  registered_address?: InputMaybe<Scalars['String']>;
  subdomain?: InputMaybe<Scalars['String']>;
  token_name?: InputMaybe<Scalars['String']>;
};

/** order by sum() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Sum_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** order by var_pop() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Var_Pop_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** order by var_samp() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Var_Samp_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** order by variance() on columns of table "current_aptos_names" */
export type Current_Aptos_Names_Variance_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "current_coin_balances" */
export type Current_Coin_Balances = {
  __typename?: 'current_coin_balances';
  amount: Scalars['numeric'];
  /** An object relationship */
  coin_info?: Maybe<Coin_Infos>;
  coin_type: Scalars['String'];
  coin_type_hash: Scalars['String'];
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  owner_address: Scalars['String'];
};

/** Boolean expression to filter rows from the table "current_coin_balances". All fields are combined with a logical 'AND'. */
export type Current_Coin_Balances_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Coin_Balances_Bool_Exp>>;
  _not?: InputMaybe<Current_Coin_Balances_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Coin_Balances_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  coin_info?: InputMaybe<Coin_Infos_Bool_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  coin_type_hash?: InputMaybe<String_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_coin_balances". */
export type Current_Coin_Balances_Order_By = {
  amount?: InputMaybe<Order_By>;
  coin_info?: InputMaybe<Coin_Infos_Order_By>;
  coin_type?: InputMaybe<Order_By>;
  coin_type_hash?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
};

/** select columns of table "current_coin_balances" */
export enum Current_Coin_Balances_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CoinTypeHash = 'coin_type_hash',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  OwnerAddress = 'owner_address'
}

/** Streaming cursor of the table "current_coin_balances" */
export type Current_Coin_Balances_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Coin_Balances_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Coin_Balances_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  coin_type?: InputMaybe<Scalars['String']>;
  coin_type_hash?: InputMaybe<Scalars['String']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  owner_address?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_collection_datas" */
export type Current_Collection_Datas = {
  __typename?: 'current_collection_datas';
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  description: Scalars['String'];
  description_mutable: Scalars['Boolean'];
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  maximum: Scalars['numeric'];
  maximum_mutable: Scalars['Boolean'];
  metadata_uri: Scalars['String'];
  supply: Scalars['numeric'];
  table_handle: Scalars['String'];
  uri_mutable: Scalars['Boolean'];
};

/** Boolean expression to filter rows from the table "current_collection_datas". All fields are combined with a logical 'AND'. */
export type Current_Collection_Datas_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Collection_Datas_Bool_Exp>>;
  _not?: InputMaybe<Current_Collection_Datas_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Collection_Datas_Bool_Exp>>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  description_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  maximum?: InputMaybe<Numeric_Comparison_Exp>;
  maximum_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  metadata_uri?: InputMaybe<String_Comparison_Exp>;
  supply?: InputMaybe<Numeric_Comparison_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
  uri_mutable?: InputMaybe<Boolean_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_collection_datas". */
export type Current_Collection_Datas_Order_By = {
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  description_mutable?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  maximum?: InputMaybe<Order_By>;
  maximum_mutable?: InputMaybe<Order_By>;
  metadata_uri?: InputMaybe<Order_By>;
  supply?: InputMaybe<Order_By>;
  table_handle?: InputMaybe<Order_By>;
  uri_mutable?: InputMaybe<Order_By>;
};

/** select columns of table "current_collection_datas" */
export enum Current_Collection_Datas_Select_Column {
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  Description = 'description',
  /** column name */
  DescriptionMutable = 'description_mutable',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Maximum = 'maximum',
  /** column name */
  MaximumMutable = 'maximum_mutable',
  /** column name */
  MetadataUri = 'metadata_uri',
  /** column name */
  Supply = 'supply',
  /** column name */
  TableHandle = 'table_handle',
  /** column name */
  UriMutable = 'uri_mutable'
}

/** Streaming cursor of the table "current_collection_datas" */
export type Current_Collection_Datas_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Collection_Datas_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Collection_Datas_Stream_Cursor_Value_Input = {
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  description?: InputMaybe<Scalars['String']>;
  description_mutable?: InputMaybe<Scalars['Boolean']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  maximum?: InputMaybe<Scalars['numeric']>;
  maximum_mutable?: InputMaybe<Scalars['Boolean']>;
  metadata_uri?: InputMaybe<Scalars['String']>;
  supply?: InputMaybe<Scalars['numeric']>;
  table_handle?: InputMaybe<Scalars['String']>;
  uri_mutable?: InputMaybe<Scalars['Boolean']>;
};

/** columns and relationships of "current_collection_ownership_v2_view" */
export type Current_Collection_Ownership_V2_View = {
  __typename?: 'current_collection_ownership_v2_view';
  collection_id?: Maybe<Scalars['String']>;
  collection_name?: Maybe<Scalars['String']>;
  collection_uri?: Maybe<Scalars['String']>;
  creator_address?: Maybe<Scalars['String']>;
  /** An object relationship */
  current_collection?: Maybe<Current_Collections_V2>;
  distinct_tokens?: Maybe<Scalars['bigint']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  single_token_uri?: Maybe<Scalars['String']>;
};

/** aggregated selection of "current_collection_ownership_v2_view" */
export type Current_Collection_Ownership_V2_View_Aggregate = {
  __typename?: 'current_collection_ownership_v2_view_aggregate';
  aggregate?: Maybe<Current_Collection_Ownership_V2_View_Aggregate_Fields>;
  nodes: Array<Current_Collection_Ownership_V2_View>;
};

/** aggregate fields of "current_collection_ownership_v2_view" */
export type Current_Collection_Ownership_V2_View_Aggregate_Fields = {
  __typename?: 'current_collection_ownership_v2_view_aggregate_fields';
  avg?: Maybe<Current_Collection_Ownership_V2_View_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Current_Collection_Ownership_V2_View_Max_Fields>;
  min?: Maybe<Current_Collection_Ownership_V2_View_Min_Fields>;
  stddev?: Maybe<Current_Collection_Ownership_V2_View_Stddev_Fields>;
  stddev_pop?: Maybe<Current_Collection_Ownership_V2_View_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Current_Collection_Ownership_V2_View_Stddev_Samp_Fields>;
  sum?: Maybe<Current_Collection_Ownership_V2_View_Sum_Fields>;
  var_pop?: Maybe<Current_Collection_Ownership_V2_View_Var_Pop_Fields>;
  var_samp?: Maybe<Current_Collection_Ownership_V2_View_Var_Samp_Fields>;
  variance?: Maybe<Current_Collection_Ownership_V2_View_Variance_Fields>;
};


/** aggregate fields of "current_collection_ownership_v2_view" */
export type Current_Collection_Ownership_V2_View_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** aggregate avg on columns */
export type Current_Collection_Ownership_V2_View_Avg_Fields = {
  __typename?: 'current_collection_ownership_v2_view_avg_fields';
  distinct_tokens?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to filter rows from the table "current_collection_ownership_v2_view". All fields are combined with a logical 'AND'. */
export type Current_Collection_Ownership_V2_View_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Bool_Exp>>;
  _not?: InputMaybe<Current_Collection_Ownership_V2_View_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Bool_Exp>>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  collection_uri?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  current_collection?: InputMaybe<Current_Collections_V2_Bool_Exp>;
  distinct_tokens?: InputMaybe<Bigint_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  single_token_uri?: InputMaybe<String_Comparison_Exp>;
};

/** aggregate max on columns */
export type Current_Collection_Ownership_V2_View_Max_Fields = {
  __typename?: 'current_collection_ownership_v2_view_max_fields';
  collection_id?: Maybe<Scalars['String']>;
  collection_name?: Maybe<Scalars['String']>;
  collection_uri?: Maybe<Scalars['String']>;
  creator_address?: Maybe<Scalars['String']>;
  distinct_tokens?: Maybe<Scalars['bigint']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  single_token_uri?: Maybe<Scalars['String']>;
};

/** aggregate min on columns */
export type Current_Collection_Ownership_V2_View_Min_Fields = {
  __typename?: 'current_collection_ownership_v2_view_min_fields';
  collection_id?: Maybe<Scalars['String']>;
  collection_name?: Maybe<Scalars['String']>;
  collection_uri?: Maybe<Scalars['String']>;
  creator_address?: Maybe<Scalars['String']>;
  distinct_tokens?: Maybe<Scalars['bigint']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  single_token_uri?: Maybe<Scalars['String']>;
};

/** Ordering options when selecting data from "current_collection_ownership_v2_view". */
export type Current_Collection_Ownership_V2_View_Order_By = {
  collection_id?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  collection_uri?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  current_collection?: InputMaybe<Current_Collections_V2_Order_By>;
  distinct_tokens?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  single_token_uri?: InputMaybe<Order_By>;
};

/** select columns of table "current_collection_ownership_v2_view" */
export enum Current_Collection_Ownership_V2_View_Select_Column {
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CollectionUri = 'collection_uri',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  DistinctTokens = 'distinct_tokens',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  SingleTokenUri = 'single_token_uri'
}

/** aggregate stddev on columns */
export type Current_Collection_Ownership_V2_View_Stddev_Fields = {
  __typename?: 'current_collection_ownership_v2_view_stddev_fields';
  distinct_tokens?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_pop on columns */
export type Current_Collection_Ownership_V2_View_Stddev_Pop_Fields = {
  __typename?: 'current_collection_ownership_v2_view_stddev_pop_fields';
  distinct_tokens?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_samp on columns */
export type Current_Collection_Ownership_V2_View_Stddev_Samp_Fields = {
  __typename?: 'current_collection_ownership_v2_view_stddev_samp_fields';
  distinct_tokens?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** Streaming cursor of the table "current_collection_ownership_v2_view" */
export type Current_Collection_Ownership_V2_View_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Collection_Ownership_V2_View_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Collection_Ownership_V2_View_Stream_Cursor_Value_Input = {
  collection_id?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  collection_uri?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  distinct_tokens?: InputMaybe<Scalars['bigint']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  owner_address?: InputMaybe<Scalars['String']>;
  single_token_uri?: InputMaybe<Scalars['String']>;
};

/** aggregate sum on columns */
export type Current_Collection_Ownership_V2_View_Sum_Fields = {
  __typename?: 'current_collection_ownership_v2_view_sum_fields';
  distinct_tokens?: Maybe<Scalars['bigint']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate var_pop on columns */
export type Current_Collection_Ownership_V2_View_Var_Pop_Fields = {
  __typename?: 'current_collection_ownership_v2_view_var_pop_fields';
  distinct_tokens?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate var_samp on columns */
export type Current_Collection_Ownership_V2_View_Var_Samp_Fields = {
  __typename?: 'current_collection_ownership_v2_view_var_samp_fields';
  distinct_tokens?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate variance on columns */
export type Current_Collection_Ownership_V2_View_Variance_Fields = {
  __typename?: 'current_collection_ownership_v2_view_variance_fields';
  distinct_tokens?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** columns and relationships of "current_collections_v2" */
export type Current_Collections_V2 = {
  __typename?: 'current_collections_v2';
  collection_id: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  current_supply: Scalars['numeric'];
  description: Scalars['String'];
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  max_supply?: Maybe<Scalars['numeric']>;
  mutable_description?: Maybe<Scalars['Boolean']>;
  mutable_uri?: Maybe<Scalars['Boolean']>;
  table_handle_v1?: Maybe<Scalars['String']>;
  token_standard: Scalars['String'];
  total_minted_v2?: Maybe<Scalars['numeric']>;
  uri: Scalars['String'];
};

/** Boolean expression to filter rows from the table "current_collections_v2". All fields are combined with a logical 'AND'. */
export type Current_Collections_V2_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Collections_V2_Bool_Exp>>;
  _not?: InputMaybe<Current_Collections_V2_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Collections_V2_Bool_Exp>>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  current_supply?: InputMaybe<Numeric_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  max_supply?: InputMaybe<Numeric_Comparison_Exp>;
  mutable_description?: InputMaybe<Boolean_Comparison_Exp>;
  mutable_uri?: InputMaybe<Boolean_Comparison_Exp>;
  table_handle_v1?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
  total_minted_v2?: InputMaybe<Numeric_Comparison_Exp>;
  uri?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_collections_v2". */
export type Current_Collections_V2_Order_By = {
  collection_id?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  current_supply?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  max_supply?: InputMaybe<Order_By>;
  mutable_description?: InputMaybe<Order_By>;
  mutable_uri?: InputMaybe<Order_By>;
  table_handle_v1?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  total_minted_v2?: InputMaybe<Order_By>;
  uri?: InputMaybe<Order_By>;
};

/** select columns of table "current_collections_v2" */
export enum Current_Collections_V2_Select_Column {
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  CurrentSupply = 'current_supply',
  /** column name */
  Description = 'description',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  MaxSupply = 'max_supply',
  /** column name */
  MutableDescription = 'mutable_description',
  /** column name */
  MutableUri = 'mutable_uri',
  /** column name */
  TableHandleV1 = 'table_handle_v1',
  /** column name */
  TokenStandard = 'token_standard',
  /** column name */
  TotalMintedV2 = 'total_minted_v2',
  /** column name */
  Uri = 'uri'
}

/** Streaming cursor of the table "current_collections_v2" */
export type Current_Collections_V2_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Collections_V2_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Collections_V2_Stream_Cursor_Value_Input = {
  collection_id?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  current_supply?: InputMaybe<Scalars['numeric']>;
  description?: InputMaybe<Scalars['String']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  max_supply?: InputMaybe<Scalars['numeric']>;
  mutable_description?: InputMaybe<Scalars['Boolean']>;
  mutable_uri?: InputMaybe<Scalars['Boolean']>;
  table_handle_v1?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
  total_minted_v2?: InputMaybe<Scalars['numeric']>;
  uri?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_delegated_staking_pool_balances" */
export type Current_Delegated_Staking_Pool_Balances = {
  __typename?: 'current_delegated_staking_pool_balances';
  active_table_handle: Scalars['String'];
  inactive_table_handle: Scalars['String'];
  last_transaction_version: Scalars['bigint'];
  operator_commission_percentage: Scalars['numeric'];
  staking_pool_address: Scalars['String'];
  total_coins: Scalars['numeric'];
  total_shares: Scalars['numeric'];
};

/** Boolean expression to filter rows from the table "current_delegated_staking_pool_balances". All fields are combined with a logical 'AND'. */
export type Current_Delegated_Staking_Pool_Balances_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Delegated_Staking_Pool_Balances_Bool_Exp>>;
  _not?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Delegated_Staking_Pool_Balances_Bool_Exp>>;
  active_table_handle?: InputMaybe<String_Comparison_Exp>;
  inactive_table_handle?: InputMaybe<String_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  operator_commission_percentage?: InputMaybe<Numeric_Comparison_Exp>;
  staking_pool_address?: InputMaybe<String_Comparison_Exp>;
  total_coins?: InputMaybe<Numeric_Comparison_Exp>;
  total_shares?: InputMaybe<Numeric_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_delegated_staking_pool_balances". */
export type Current_Delegated_Staking_Pool_Balances_Order_By = {
  active_table_handle?: InputMaybe<Order_By>;
  inactive_table_handle?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  operator_commission_percentage?: InputMaybe<Order_By>;
  staking_pool_address?: InputMaybe<Order_By>;
  total_coins?: InputMaybe<Order_By>;
  total_shares?: InputMaybe<Order_By>;
};

/** select columns of table "current_delegated_staking_pool_balances" */
export enum Current_Delegated_Staking_Pool_Balances_Select_Column {
  /** column name */
  ActiveTableHandle = 'active_table_handle',
  /** column name */
  InactiveTableHandle = 'inactive_table_handle',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  OperatorCommissionPercentage = 'operator_commission_percentage',
  /** column name */
  StakingPoolAddress = 'staking_pool_address',
  /** column name */
  TotalCoins = 'total_coins',
  /** column name */
  TotalShares = 'total_shares'
}

/** Streaming cursor of the table "current_delegated_staking_pool_balances" */
export type Current_Delegated_Staking_Pool_Balances_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Delegated_Staking_Pool_Balances_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Delegated_Staking_Pool_Balances_Stream_Cursor_Value_Input = {
  active_table_handle?: InputMaybe<Scalars['String']>;
  inactive_table_handle?: InputMaybe<Scalars['String']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  operator_commission_percentage?: InputMaybe<Scalars['numeric']>;
  staking_pool_address?: InputMaybe<Scalars['String']>;
  total_coins?: InputMaybe<Scalars['numeric']>;
  total_shares?: InputMaybe<Scalars['numeric']>;
};

/** columns and relationships of "current_delegated_voter" */
export type Current_Delegated_Voter = {
  __typename?: 'current_delegated_voter';
  delegation_pool_address: Scalars['String'];
  delegator_address: Scalars['String'];
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  pending_voter?: Maybe<Scalars['String']>;
  table_handle?: Maybe<Scalars['String']>;
  voter?: Maybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "current_delegated_voter". All fields are combined with a logical 'AND'. */
export type Current_Delegated_Voter_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Delegated_Voter_Bool_Exp>>;
  _not?: InputMaybe<Current_Delegated_Voter_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Delegated_Voter_Bool_Exp>>;
  delegation_pool_address?: InputMaybe<String_Comparison_Exp>;
  delegator_address?: InputMaybe<String_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  pending_voter?: InputMaybe<String_Comparison_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
  voter?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_delegated_voter". */
export type Current_Delegated_Voter_Order_By = {
  delegation_pool_address?: InputMaybe<Order_By>;
  delegator_address?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  pending_voter?: InputMaybe<Order_By>;
  table_handle?: InputMaybe<Order_By>;
  voter?: InputMaybe<Order_By>;
};

/** select columns of table "current_delegated_voter" */
export enum Current_Delegated_Voter_Select_Column {
  /** column name */
  DelegationPoolAddress = 'delegation_pool_address',
  /** column name */
  DelegatorAddress = 'delegator_address',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  PendingVoter = 'pending_voter',
  /** column name */
  TableHandle = 'table_handle',
  /** column name */
  Voter = 'voter'
}

/** Streaming cursor of the table "current_delegated_voter" */
export type Current_Delegated_Voter_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Delegated_Voter_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Delegated_Voter_Stream_Cursor_Value_Input = {
  delegation_pool_address?: InputMaybe<Scalars['String']>;
  delegator_address?: InputMaybe<Scalars['String']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  pending_voter?: InputMaybe<Scalars['String']>;
  table_handle?: InputMaybe<Scalars['String']>;
  voter?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_delegator_balances" */
export type Current_Delegator_Balances = {
  __typename?: 'current_delegator_balances';
  /** An object relationship */
  current_pool_balance?: Maybe<Current_Delegated_Staking_Pool_Balances>;
  delegator_address: Scalars['String'];
  last_transaction_version: Scalars['bigint'];
  parent_table_handle: Scalars['String'];
  pool_address: Scalars['String'];
  pool_type: Scalars['String'];
  shares: Scalars['numeric'];
  /** An object relationship */
  staking_pool_metadata?: Maybe<Current_Staking_Pool_Voter>;
  table_handle: Scalars['String'];
};

/** Boolean expression to filter rows from the table "current_delegator_balances". All fields are combined with a logical 'AND'. */
export type Current_Delegator_Balances_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Delegator_Balances_Bool_Exp>>;
  _not?: InputMaybe<Current_Delegator_Balances_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Delegator_Balances_Bool_Exp>>;
  current_pool_balance?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Bool_Exp>;
  delegator_address?: InputMaybe<String_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  parent_table_handle?: InputMaybe<String_Comparison_Exp>;
  pool_address?: InputMaybe<String_Comparison_Exp>;
  pool_type?: InputMaybe<String_Comparison_Exp>;
  shares?: InputMaybe<Numeric_Comparison_Exp>;
  staking_pool_metadata?: InputMaybe<Current_Staking_Pool_Voter_Bool_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_delegator_balances". */
export type Current_Delegator_Balances_Order_By = {
  current_pool_balance?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Order_By>;
  delegator_address?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  parent_table_handle?: InputMaybe<Order_By>;
  pool_address?: InputMaybe<Order_By>;
  pool_type?: InputMaybe<Order_By>;
  shares?: InputMaybe<Order_By>;
  staking_pool_metadata?: InputMaybe<Current_Staking_Pool_Voter_Order_By>;
  table_handle?: InputMaybe<Order_By>;
};

/** select columns of table "current_delegator_balances" */
export enum Current_Delegator_Balances_Select_Column {
  /** column name */
  DelegatorAddress = 'delegator_address',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  ParentTableHandle = 'parent_table_handle',
  /** column name */
  PoolAddress = 'pool_address',
  /** column name */
  PoolType = 'pool_type',
  /** column name */
  Shares = 'shares',
  /** column name */
  TableHandle = 'table_handle'
}

/** Streaming cursor of the table "current_delegator_balances" */
export type Current_Delegator_Balances_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Delegator_Balances_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Delegator_Balances_Stream_Cursor_Value_Input = {
  delegator_address?: InputMaybe<Scalars['String']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  parent_table_handle?: InputMaybe<Scalars['String']>;
  pool_address?: InputMaybe<Scalars['String']>;
  pool_type?: InputMaybe<Scalars['String']>;
  shares?: InputMaybe<Scalars['numeric']>;
  table_handle?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_fungible_asset_balances" */
export type Current_Fungible_Asset_Balances = {
  __typename?: 'current_fungible_asset_balances';
  amount: Scalars['numeric'];
  asset_type: Scalars['String'];
  is_frozen: Scalars['Boolean'];
  is_primary: Scalars['Boolean'];
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  /** An object relationship */
  metadata?: Maybe<Fungible_Asset_Metadata>;
  owner_address: Scalars['String'];
  storage_id: Scalars['String'];
  token_standard: Scalars['String'];
};

/** aggregated selection of "current_fungible_asset_balances" */
export type Current_Fungible_Asset_Balances_Aggregate = {
  __typename?: 'current_fungible_asset_balances_aggregate';
  aggregate?: Maybe<Current_Fungible_Asset_Balances_Aggregate_Fields>;
  nodes: Array<Current_Fungible_Asset_Balances>;
};

/** aggregate fields of "current_fungible_asset_balances" */
export type Current_Fungible_Asset_Balances_Aggregate_Fields = {
  __typename?: 'current_fungible_asset_balances_aggregate_fields';
  avg?: Maybe<Current_Fungible_Asset_Balances_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Current_Fungible_Asset_Balances_Max_Fields>;
  min?: Maybe<Current_Fungible_Asset_Balances_Min_Fields>;
  stddev?: Maybe<Current_Fungible_Asset_Balances_Stddev_Fields>;
  stddev_pop?: Maybe<Current_Fungible_Asset_Balances_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Current_Fungible_Asset_Balances_Stddev_Samp_Fields>;
  sum?: Maybe<Current_Fungible_Asset_Balances_Sum_Fields>;
  var_pop?: Maybe<Current_Fungible_Asset_Balances_Var_Pop_Fields>;
  var_samp?: Maybe<Current_Fungible_Asset_Balances_Var_Samp_Fields>;
  variance?: Maybe<Current_Fungible_Asset_Balances_Variance_Fields>;
};


/** aggregate fields of "current_fungible_asset_balances" */
export type Current_Fungible_Asset_Balances_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Current_Fungible_Asset_Balances_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** aggregate avg on columns */
export type Current_Fungible_Asset_Balances_Avg_Fields = {
  __typename?: 'current_fungible_asset_balances_avg_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to filter rows from the table "current_fungible_asset_balances". All fields are combined with a logical 'AND'. */
export type Current_Fungible_Asset_Balances_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Fungible_Asset_Balances_Bool_Exp>>;
  _not?: InputMaybe<Current_Fungible_Asset_Balances_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Fungible_Asset_Balances_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  asset_type?: InputMaybe<String_Comparison_Exp>;
  is_frozen?: InputMaybe<Boolean_Comparison_Exp>;
  is_primary?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  metadata?: InputMaybe<Fungible_Asset_Metadata_Bool_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  storage_id?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
};

/** aggregate max on columns */
export type Current_Fungible_Asset_Balances_Max_Fields = {
  __typename?: 'current_fungible_asset_balances_max_fields';
  amount?: Maybe<Scalars['numeric']>;
  asset_type?: Maybe<Scalars['String']>;
  last_transaction_timestamp?: Maybe<Scalars['timestamp']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  storage_id?: Maybe<Scalars['String']>;
  token_standard?: Maybe<Scalars['String']>;
};

/** aggregate min on columns */
export type Current_Fungible_Asset_Balances_Min_Fields = {
  __typename?: 'current_fungible_asset_balances_min_fields';
  amount?: Maybe<Scalars['numeric']>;
  asset_type?: Maybe<Scalars['String']>;
  last_transaction_timestamp?: Maybe<Scalars['timestamp']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  storage_id?: Maybe<Scalars['String']>;
  token_standard?: Maybe<Scalars['String']>;
};

/** Ordering options when selecting data from "current_fungible_asset_balances". */
export type Current_Fungible_Asset_Balances_Order_By = {
  amount?: InputMaybe<Order_By>;
  asset_type?: InputMaybe<Order_By>;
  is_frozen?: InputMaybe<Order_By>;
  is_primary?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  metadata?: InputMaybe<Fungible_Asset_Metadata_Order_By>;
  owner_address?: InputMaybe<Order_By>;
  storage_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** select columns of table "current_fungible_asset_balances" */
export enum Current_Fungible_Asset_Balances_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  AssetType = 'asset_type',
  /** column name */
  IsFrozen = 'is_frozen',
  /** column name */
  IsPrimary = 'is_primary',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  StorageId = 'storage_id',
  /** column name */
  TokenStandard = 'token_standard'
}

/** aggregate stddev on columns */
export type Current_Fungible_Asset_Balances_Stddev_Fields = {
  __typename?: 'current_fungible_asset_balances_stddev_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_pop on columns */
export type Current_Fungible_Asset_Balances_Stddev_Pop_Fields = {
  __typename?: 'current_fungible_asset_balances_stddev_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_samp on columns */
export type Current_Fungible_Asset_Balances_Stddev_Samp_Fields = {
  __typename?: 'current_fungible_asset_balances_stddev_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** Streaming cursor of the table "current_fungible_asset_balances" */
export type Current_Fungible_Asset_Balances_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Fungible_Asset_Balances_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Fungible_Asset_Balances_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  asset_type?: InputMaybe<Scalars['String']>;
  is_frozen?: InputMaybe<Scalars['Boolean']>;
  is_primary?: InputMaybe<Scalars['Boolean']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  owner_address?: InputMaybe<Scalars['String']>;
  storage_id?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
};

/** aggregate sum on columns */
export type Current_Fungible_Asset_Balances_Sum_Fields = {
  __typename?: 'current_fungible_asset_balances_sum_fields';
  amount?: Maybe<Scalars['numeric']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate var_pop on columns */
export type Current_Fungible_Asset_Balances_Var_Pop_Fields = {
  __typename?: 'current_fungible_asset_balances_var_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate var_samp on columns */
export type Current_Fungible_Asset_Balances_Var_Samp_Fields = {
  __typename?: 'current_fungible_asset_balances_var_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate variance on columns */
export type Current_Fungible_Asset_Balances_Variance_Fields = {
  __typename?: 'current_fungible_asset_balances_variance_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
};

/** columns and relationships of "current_objects" */
export type Current_Objects = {
  __typename?: 'current_objects';
  allow_ungated_transfer: Scalars['Boolean'];
  is_deleted: Scalars['Boolean'];
  last_guid_creation_num: Scalars['numeric'];
  last_transaction_version: Scalars['bigint'];
  object_address: Scalars['String'];
  owner_address: Scalars['String'];
  state_key_hash: Scalars['String'];
};

/** Boolean expression to filter rows from the table "current_objects". All fields are combined with a logical 'AND'. */
export type Current_Objects_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Objects_Bool_Exp>>;
  _not?: InputMaybe<Current_Objects_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Objects_Bool_Exp>>;
  allow_ungated_transfer?: InputMaybe<Boolean_Comparison_Exp>;
  is_deleted?: InputMaybe<Boolean_Comparison_Exp>;
  last_guid_creation_num?: InputMaybe<Numeric_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  object_address?: InputMaybe<String_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  state_key_hash?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_objects". */
export type Current_Objects_Order_By = {
  allow_ungated_transfer?: InputMaybe<Order_By>;
  is_deleted?: InputMaybe<Order_By>;
  last_guid_creation_num?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  object_address?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  state_key_hash?: InputMaybe<Order_By>;
};

/** select columns of table "current_objects" */
export enum Current_Objects_Select_Column {
  /** column name */
  AllowUngatedTransfer = 'allow_ungated_transfer',
  /** column name */
  IsDeleted = 'is_deleted',
  /** column name */
  LastGuidCreationNum = 'last_guid_creation_num',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  ObjectAddress = 'object_address',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  StateKeyHash = 'state_key_hash'
}

/** Streaming cursor of the table "current_objects" */
export type Current_Objects_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Objects_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Objects_Stream_Cursor_Value_Input = {
  allow_ungated_transfer?: InputMaybe<Scalars['Boolean']>;
  is_deleted?: InputMaybe<Scalars['Boolean']>;
  last_guid_creation_num?: InputMaybe<Scalars['numeric']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  object_address?: InputMaybe<Scalars['String']>;
  owner_address?: InputMaybe<Scalars['String']>;
  state_key_hash?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_staking_pool_voter" */
export type Current_Staking_Pool_Voter = {
  __typename?: 'current_staking_pool_voter';
  last_transaction_version: Scalars['bigint'];
  operator_address: Scalars['String'];
  /** An array relationship */
  operator_aptos_name: Array<Current_Aptos_Names>;
  staking_pool_address: Scalars['String'];
  voter_address: Scalars['String'];
};


/** columns and relationships of "current_staking_pool_voter" */
export type Current_Staking_Pool_VoterOperator_Aptos_NameArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};

/** Boolean expression to filter rows from the table "current_staking_pool_voter". All fields are combined with a logical 'AND'. */
export type Current_Staking_Pool_Voter_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Staking_Pool_Voter_Bool_Exp>>;
  _not?: InputMaybe<Current_Staking_Pool_Voter_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Staking_Pool_Voter_Bool_Exp>>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  operator_address?: InputMaybe<String_Comparison_Exp>;
  operator_aptos_name?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  staking_pool_address?: InputMaybe<String_Comparison_Exp>;
  voter_address?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_staking_pool_voter". */
export type Current_Staking_Pool_Voter_Order_By = {
  last_transaction_version?: InputMaybe<Order_By>;
  operator_address?: InputMaybe<Order_By>;
  operator_aptos_name_aggregate?: InputMaybe<Current_Aptos_Names_Aggregate_Order_By>;
  staking_pool_address?: InputMaybe<Order_By>;
  voter_address?: InputMaybe<Order_By>;
};

/** select columns of table "current_staking_pool_voter" */
export enum Current_Staking_Pool_Voter_Select_Column {
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  OperatorAddress = 'operator_address',
  /** column name */
  StakingPoolAddress = 'staking_pool_address',
  /** column name */
  VoterAddress = 'voter_address'
}

/** Streaming cursor of the table "current_staking_pool_voter" */
export type Current_Staking_Pool_Voter_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Staking_Pool_Voter_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Staking_Pool_Voter_Stream_Cursor_Value_Input = {
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  operator_address?: InputMaybe<Scalars['String']>;
  staking_pool_address?: InputMaybe<Scalars['String']>;
  voter_address?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_table_items" */
export type Current_Table_Items = {
  __typename?: 'current_table_items';
  decoded_key: Scalars['jsonb'];
  decoded_value?: Maybe<Scalars['jsonb']>;
  is_deleted: Scalars['Boolean'];
  key: Scalars['String'];
  key_hash: Scalars['String'];
  last_transaction_version: Scalars['bigint'];
  table_handle: Scalars['String'];
};


/** columns and relationships of "current_table_items" */
export type Current_Table_ItemsDecoded_KeyArgs = {
  path?: InputMaybe<Scalars['String']>;
};


/** columns and relationships of "current_table_items" */
export type Current_Table_ItemsDecoded_ValueArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "current_table_items". All fields are combined with a logical 'AND'. */
export type Current_Table_Items_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Table_Items_Bool_Exp>>;
  _not?: InputMaybe<Current_Table_Items_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Table_Items_Bool_Exp>>;
  decoded_key?: InputMaybe<Jsonb_Comparison_Exp>;
  decoded_value?: InputMaybe<Jsonb_Comparison_Exp>;
  is_deleted?: InputMaybe<Boolean_Comparison_Exp>;
  key?: InputMaybe<String_Comparison_Exp>;
  key_hash?: InputMaybe<String_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_table_items". */
export type Current_Table_Items_Order_By = {
  decoded_key?: InputMaybe<Order_By>;
  decoded_value?: InputMaybe<Order_By>;
  is_deleted?: InputMaybe<Order_By>;
  key?: InputMaybe<Order_By>;
  key_hash?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  table_handle?: InputMaybe<Order_By>;
};

/** select columns of table "current_table_items" */
export enum Current_Table_Items_Select_Column {
  /** column name */
  DecodedKey = 'decoded_key',
  /** column name */
  DecodedValue = 'decoded_value',
  /** column name */
  IsDeleted = 'is_deleted',
  /** column name */
  Key = 'key',
  /** column name */
  KeyHash = 'key_hash',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  TableHandle = 'table_handle'
}

/** Streaming cursor of the table "current_table_items" */
export type Current_Table_Items_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Table_Items_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Table_Items_Stream_Cursor_Value_Input = {
  decoded_key?: InputMaybe<Scalars['jsonb']>;
  decoded_value?: InputMaybe<Scalars['jsonb']>;
  is_deleted?: InputMaybe<Scalars['Boolean']>;
  key?: InputMaybe<Scalars['String']>;
  key_hash?: InputMaybe<Scalars['String']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  table_handle?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_token_datas" */
export type Current_Token_Datas = {
  __typename?: 'current_token_datas';
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  /** An object relationship */
  current_collection_data?: Maybe<Current_Collection_Datas>;
  default_properties: Scalars['jsonb'];
  description: Scalars['String'];
  description_mutable: Scalars['Boolean'];
  largest_property_version: Scalars['numeric'];
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  maximum: Scalars['numeric'];
  maximum_mutable: Scalars['Boolean'];
  metadata_uri: Scalars['String'];
  name: Scalars['String'];
  payee_address: Scalars['String'];
  properties_mutable: Scalars['Boolean'];
  royalty_mutable: Scalars['Boolean'];
  royalty_points_denominator: Scalars['numeric'];
  royalty_points_numerator: Scalars['numeric'];
  supply: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
  uri_mutable: Scalars['Boolean'];
};


/** columns and relationships of "current_token_datas" */
export type Current_Token_DatasDefault_PropertiesArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "current_token_datas". All fields are combined with a logical 'AND'. */
export type Current_Token_Datas_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Token_Datas_Bool_Exp>>;
  _not?: InputMaybe<Current_Token_Datas_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Token_Datas_Bool_Exp>>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  current_collection_data?: InputMaybe<Current_Collection_Datas_Bool_Exp>;
  default_properties?: InputMaybe<Jsonb_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  description_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  largest_property_version?: InputMaybe<Numeric_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  maximum?: InputMaybe<Numeric_Comparison_Exp>;
  maximum_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  metadata_uri?: InputMaybe<String_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  payee_address?: InputMaybe<String_Comparison_Exp>;
  properties_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  royalty_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  royalty_points_denominator?: InputMaybe<Numeric_Comparison_Exp>;
  royalty_points_numerator?: InputMaybe<Numeric_Comparison_Exp>;
  supply?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  uri_mutable?: InputMaybe<Boolean_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_token_datas". */
export type Current_Token_Datas_Order_By = {
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  current_collection_data?: InputMaybe<Current_Collection_Datas_Order_By>;
  default_properties?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  description_mutable?: InputMaybe<Order_By>;
  largest_property_version?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  maximum?: InputMaybe<Order_By>;
  maximum_mutable?: InputMaybe<Order_By>;
  metadata_uri?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  payee_address?: InputMaybe<Order_By>;
  properties_mutable?: InputMaybe<Order_By>;
  royalty_mutable?: InputMaybe<Order_By>;
  royalty_points_denominator?: InputMaybe<Order_By>;
  royalty_points_numerator?: InputMaybe<Order_By>;
  supply?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  uri_mutable?: InputMaybe<Order_By>;
};

/** select columns of table "current_token_datas" */
export enum Current_Token_Datas_Select_Column {
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  DefaultProperties = 'default_properties',
  /** column name */
  Description = 'description',
  /** column name */
  DescriptionMutable = 'description_mutable',
  /** column name */
  LargestPropertyVersion = 'largest_property_version',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Maximum = 'maximum',
  /** column name */
  MaximumMutable = 'maximum_mutable',
  /** column name */
  MetadataUri = 'metadata_uri',
  /** column name */
  Name = 'name',
  /** column name */
  PayeeAddress = 'payee_address',
  /** column name */
  PropertiesMutable = 'properties_mutable',
  /** column name */
  RoyaltyMutable = 'royalty_mutable',
  /** column name */
  RoyaltyPointsDenominator = 'royalty_points_denominator',
  /** column name */
  RoyaltyPointsNumerator = 'royalty_points_numerator',
  /** column name */
  Supply = 'supply',
  /** column name */
  TokenDataIdHash = 'token_data_id_hash',
  /** column name */
  UriMutable = 'uri_mutable'
}

/** Streaming cursor of the table "current_token_datas" */
export type Current_Token_Datas_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Token_Datas_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Token_Datas_Stream_Cursor_Value_Input = {
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  default_properties?: InputMaybe<Scalars['jsonb']>;
  description?: InputMaybe<Scalars['String']>;
  description_mutable?: InputMaybe<Scalars['Boolean']>;
  largest_property_version?: InputMaybe<Scalars['numeric']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  maximum?: InputMaybe<Scalars['numeric']>;
  maximum_mutable?: InputMaybe<Scalars['Boolean']>;
  metadata_uri?: InputMaybe<Scalars['String']>;
  name?: InputMaybe<Scalars['String']>;
  payee_address?: InputMaybe<Scalars['String']>;
  properties_mutable?: InputMaybe<Scalars['Boolean']>;
  royalty_mutable?: InputMaybe<Scalars['Boolean']>;
  royalty_points_denominator?: InputMaybe<Scalars['numeric']>;
  royalty_points_numerator?: InputMaybe<Scalars['numeric']>;
  supply?: InputMaybe<Scalars['numeric']>;
  token_data_id_hash?: InputMaybe<Scalars['String']>;
  uri_mutable?: InputMaybe<Scalars['Boolean']>;
};

/** columns and relationships of "current_token_datas_v2" */
export type Current_Token_Datas_V2 = {
  __typename?: 'current_token_datas_v2';
  /** An object relationship */
  aptos_name?: Maybe<Current_Aptos_Names>;
  /** An object relationship */
  cdn_token_uris?: Maybe<Nft_Metadata_Crawler_Parsed_Asset_Uris>;
  collection_id: Scalars['String'];
  /** An object relationship */
  current_collection?: Maybe<Current_Collections_V2>;
  description: Scalars['String'];
  is_fungible_v2?: Maybe<Scalars['Boolean']>;
  largest_property_version_v1?: Maybe<Scalars['numeric']>;
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  maximum?: Maybe<Scalars['numeric']>;
  supply: Scalars['numeric'];
  token_data_id: Scalars['String'];
  token_name: Scalars['String'];
  token_properties: Scalars['jsonb'];
  token_standard: Scalars['String'];
  token_uri: Scalars['String'];
};


/** columns and relationships of "current_token_datas_v2" */
export type Current_Token_Datas_V2Token_PropertiesArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "current_token_datas_v2". All fields are combined with a logical 'AND'. */
export type Current_Token_Datas_V2_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Token_Datas_V2_Bool_Exp>>;
  _not?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Token_Datas_V2_Bool_Exp>>;
  aptos_name?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  cdn_token_uris?: InputMaybe<Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  current_collection?: InputMaybe<Current_Collections_V2_Bool_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  is_fungible_v2?: InputMaybe<Boolean_Comparison_Exp>;
  largest_property_version_v1?: InputMaybe<Numeric_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  maximum?: InputMaybe<Numeric_Comparison_Exp>;
  supply?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_name?: InputMaybe<String_Comparison_Exp>;
  token_properties?: InputMaybe<Jsonb_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
  token_uri?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_token_datas_v2". */
export type Current_Token_Datas_V2_Order_By = {
  aptos_name?: InputMaybe<Current_Aptos_Names_Order_By>;
  cdn_token_uris?: InputMaybe<Nft_Metadata_Crawler_Parsed_Asset_Uris_Order_By>;
  collection_id?: InputMaybe<Order_By>;
  current_collection?: InputMaybe<Current_Collections_V2_Order_By>;
  description?: InputMaybe<Order_By>;
  is_fungible_v2?: InputMaybe<Order_By>;
  largest_property_version_v1?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  maximum?: InputMaybe<Order_By>;
  supply?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_name?: InputMaybe<Order_By>;
  token_properties?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  token_uri?: InputMaybe<Order_By>;
};

/** select columns of table "current_token_datas_v2" */
export enum Current_Token_Datas_V2_Select_Column {
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  Description = 'description',
  /** column name */
  IsFungibleV2 = 'is_fungible_v2',
  /** column name */
  LargestPropertyVersionV1 = 'largest_property_version_v1',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Maximum = 'maximum',
  /** column name */
  Supply = 'supply',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenName = 'token_name',
  /** column name */
  TokenProperties = 'token_properties',
  /** column name */
  TokenStandard = 'token_standard',
  /** column name */
  TokenUri = 'token_uri'
}

/** Streaming cursor of the table "current_token_datas_v2" */
export type Current_Token_Datas_V2_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Token_Datas_V2_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Token_Datas_V2_Stream_Cursor_Value_Input = {
  collection_id?: InputMaybe<Scalars['String']>;
  description?: InputMaybe<Scalars['String']>;
  is_fungible_v2?: InputMaybe<Scalars['Boolean']>;
  largest_property_version_v1?: InputMaybe<Scalars['numeric']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  maximum?: InputMaybe<Scalars['numeric']>;
  supply?: InputMaybe<Scalars['numeric']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_name?: InputMaybe<Scalars['String']>;
  token_properties?: InputMaybe<Scalars['jsonb']>;
  token_standard?: InputMaybe<Scalars['String']>;
  token_uri?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "current_token_ownerships" */
export type Current_Token_Ownerships = {
  __typename?: 'current_token_ownerships';
  amount: Scalars['numeric'];
  /** An object relationship */
  aptos_name?: Maybe<Current_Aptos_Names>;
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  /** An object relationship */
  current_collection_data?: Maybe<Current_Collection_Datas>;
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas>;
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  name: Scalars['String'];
  owner_address: Scalars['String'];
  property_version: Scalars['numeric'];
  table_type: Scalars['String'];
  token_data_id_hash: Scalars['String'];
  token_properties: Scalars['jsonb'];
};


/** columns and relationships of "current_token_ownerships" */
export type Current_Token_OwnershipsToken_PropertiesArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** aggregated selection of "current_token_ownerships" */
export type Current_Token_Ownerships_Aggregate = {
  __typename?: 'current_token_ownerships_aggregate';
  aggregate?: Maybe<Current_Token_Ownerships_Aggregate_Fields>;
  nodes: Array<Current_Token_Ownerships>;
};

/** aggregate fields of "current_token_ownerships" */
export type Current_Token_Ownerships_Aggregate_Fields = {
  __typename?: 'current_token_ownerships_aggregate_fields';
  avg?: Maybe<Current_Token_Ownerships_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Current_Token_Ownerships_Max_Fields>;
  min?: Maybe<Current_Token_Ownerships_Min_Fields>;
  stddev?: Maybe<Current_Token_Ownerships_Stddev_Fields>;
  stddev_pop?: Maybe<Current_Token_Ownerships_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Current_Token_Ownerships_Stddev_Samp_Fields>;
  sum?: Maybe<Current_Token_Ownerships_Sum_Fields>;
  var_pop?: Maybe<Current_Token_Ownerships_Var_Pop_Fields>;
  var_samp?: Maybe<Current_Token_Ownerships_Var_Samp_Fields>;
  variance?: Maybe<Current_Token_Ownerships_Variance_Fields>;
};


/** aggregate fields of "current_token_ownerships" */
export type Current_Token_Ownerships_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Current_Token_Ownerships_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** order by aggregate values of table "current_token_ownerships" */
export type Current_Token_Ownerships_Aggregate_Order_By = {
  avg?: InputMaybe<Current_Token_Ownerships_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Current_Token_Ownerships_Max_Order_By>;
  min?: InputMaybe<Current_Token_Ownerships_Min_Order_By>;
  stddev?: InputMaybe<Current_Token_Ownerships_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Current_Token_Ownerships_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Current_Token_Ownerships_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Current_Token_Ownerships_Sum_Order_By>;
  var_pop?: InputMaybe<Current_Token_Ownerships_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Current_Token_Ownerships_Var_Samp_Order_By>;
  variance?: InputMaybe<Current_Token_Ownerships_Variance_Order_By>;
};

/** aggregate avg on columns */
export type Current_Token_Ownerships_Avg_Fields = {
  __typename?: 'current_token_ownerships_avg_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
};

/** order by avg() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Avg_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "current_token_ownerships". All fields are combined with a logical 'AND'. */
export type Current_Token_Ownerships_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Token_Ownerships_Bool_Exp>>;
  _not?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Token_Ownerships_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  aptos_name?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  current_collection_data?: InputMaybe<Current_Collection_Datas_Bool_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_Bool_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  property_version?: InputMaybe<Numeric_Comparison_Exp>;
  table_type?: InputMaybe<String_Comparison_Exp>;
  token_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  token_properties?: InputMaybe<Jsonb_Comparison_Exp>;
};

/** aggregate max on columns */
export type Current_Token_Ownerships_Max_Fields = {
  __typename?: 'current_token_ownerships_max_fields';
  amount?: Maybe<Scalars['numeric']>;
  collection_data_id_hash?: Maybe<Scalars['String']>;
  collection_name?: Maybe<Scalars['String']>;
  creator_address?: Maybe<Scalars['String']>;
  last_transaction_timestamp?: Maybe<Scalars['timestamp']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  name?: Maybe<Scalars['String']>;
  owner_address?: Maybe<Scalars['String']>;
  property_version?: Maybe<Scalars['numeric']>;
  table_type?: Maybe<Scalars['String']>;
  token_data_id_hash?: Maybe<Scalars['String']>;
};

/** order by max() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Max_Order_By = {
  amount?: InputMaybe<Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  table_type?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Current_Token_Ownerships_Min_Fields = {
  __typename?: 'current_token_ownerships_min_fields';
  amount?: Maybe<Scalars['numeric']>;
  collection_data_id_hash?: Maybe<Scalars['String']>;
  collection_name?: Maybe<Scalars['String']>;
  creator_address?: Maybe<Scalars['String']>;
  last_transaction_timestamp?: Maybe<Scalars['timestamp']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  name?: Maybe<Scalars['String']>;
  owner_address?: Maybe<Scalars['String']>;
  property_version?: Maybe<Scalars['numeric']>;
  table_type?: Maybe<Scalars['String']>;
  token_data_id_hash?: Maybe<Scalars['String']>;
};

/** order by min() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Min_Order_By = {
  amount?: InputMaybe<Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  table_type?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "current_token_ownerships". */
export type Current_Token_Ownerships_Order_By = {
  amount?: InputMaybe<Order_By>;
  aptos_name?: InputMaybe<Current_Aptos_Names_Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  current_collection_data?: InputMaybe<Current_Collection_Datas_Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  table_type?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  token_properties?: InputMaybe<Order_By>;
};

/** select columns of table "current_token_ownerships" */
export enum Current_Token_Ownerships_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Name = 'name',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  PropertyVersion = 'property_version',
  /** column name */
  TableType = 'table_type',
  /** column name */
  TokenDataIdHash = 'token_data_id_hash',
  /** column name */
  TokenProperties = 'token_properties'
}

/** aggregate stddev on columns */
export type Current_Token_Ownerships_Stddev_Fields = {
  __typename?: 'current_token_ownerships_stddev_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
};

/** order by stddev() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Stddev_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_pop on columns */
export type Current_Token_Ownerships_Stddev_Pop_Fields = {
  __typename?: 'current_token_ownerships_stddev_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_pop() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Stddev_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_samp on columns */
export type Current_Token_Ownerships_Stddev_Samp_Fields = {
  __typename?: 'current_token_ownerships_stddev_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_samp() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Stddev_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "current_token_ownerships" */
export type Current_Token_Ownerships_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Token_Ownerships_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Token_Ownerships_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  name?: InputMaybe<Scalars['String']>;
  owner_address?: InputMaybe<Scalars['String']>;
  property_version?: InputMaybe<Scalars['numeric']>;
  table_type?: InputMaybe<Scalars['String']>;
  token_data_id_hash?: InputMaybe<Scalars['String']>;
  token_properties?: InputMaybe<Scalars['jsonb']>;
};

/** aggregate sum on columns */
export type Current_Token_Ownerships_Sum_Fields = {
  __typename?: 'current_token_ownerships_sum_fields';
  amount?: Maybe<Scalars['numeric']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  property_version?: Maybe<Scalars['numeric']>;
};

/** order by sum() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Sum_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2 = {
  __typename?: 'current_token_ownerships_v2';
  amount: Scalars['numeric'];
  /** An array relationship */
  composed_nfts: Array<Current_Token_Ownerships_V2>;
  /** An aggregate relationship */
  composed_nfts_aggregate: Current_Token_Ownerships_V2_Aggregate;
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas_V2>;
  is_fungible_v2?: Maybe<Scalars['Boolean']>;
  is_soulbound_v2?: Maybe<Scalars['Boolean']>;
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  owner_address: Scalars['String'];
  property_version_v1: Scalars['numeric'];
  storage_id: Scalars['String'];
  table_type_v1?: Maybe<Scalars['String']>;
  token_data_id: Scalars['String'];
  token_properties_mutated_v1?: Maybe<Scalars['jsonb']>;
  token_standard: Scalars['String'];
};


/** columns and relationships of "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2Composed_NftsArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
};


/** columns and relationships of "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2Composed_Nfts_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
};


/** columns and relationships of "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2Token_Properties_Mutated_V1Args = {
  path?: InputMaybe<Scalars['String']>;
};

/** aggregated selection of "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Aggregate = {
  __typename?: 'current_token_ownerships_v2_aggregate';
  aggregate?: Maybe<Current_Token_Ownerships_V2_Aggregate_Fields>;
  nodes: Array<Current_Token_Ownerships_V2>;
};

/** aggregate fields of "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Aggregate_Fields = {
  __typename?: 'current_token_ownerships_v2_aggregate_fields';
  avg?: Maybe<Current_Token_Ownerships_V2_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Current_Token_Ownerships_V2_Max_Fields>;
  min?: Maybe<Current_Token_Ownerships_V2_Min_Fields>;
  stddev?: Maybe<Current_Token_Ownerships_V2_Stddev_Fields>;
  stddev_pop?: Maybe<Current_Token_Ownerships_V2_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Current_Token_Ownerships_V2_Stddev_Samp_Fields>;
  sum?: Maybe<Current_Token_Ownerships_V2_Sum_Fields>;
  var_pop?: Maybe<Current_Token_Ownerships_V2_Var_Pop_Fields>;
  var_samp?: Maybe<Current_Token_Ownerships_V2_Var_Samp_Fields>;
  variance?: Maybe<Current_Token_Ownerships_V2_Variance_Fields>;
};


/** aggregate fields of "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Current_Token_Ownerships_V2_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** order by aggregate values of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Aggregate_Order_By = {
  avg?: InputMaybe<Current_Token_Ownerships_V2_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Current_Token_Ownerships_V2_Max_Order_By>;
  min?: InputMaybe<Current_Token_Ownerships_V2_Min_Order_By>;
  stddev?: InputMaybe<Current_Token_Ownerships_V2_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Current_Token_Ownerships_V2_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Current_Token_Ownerships_V2_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Current_Token_Ownerships_V2_Sum_Order_By>;
  var_pop?: InputMaybe<Current_Token_Ownerships_V2_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Current_Token_Ownerships_V2_Var_Samp_Order_By>;
  variance?: InputMaybe<Current_Token_Ownerships_V2_Variance_Order_By>;
};

/** aggregate avg on columns */
export type Current_Token_Ownerships_V2_Avg_Fields = {
  __typename?: 'current_token_ownerships_v2_avg_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
};

/** order by avg() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Avg_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "current_token_ownerships_v2". All fields are combined with a logical 'AND'. */
export type Current_Token_Ownerships_V2_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Token_Ownerships_V2_Bool_Exp>>;
  _not?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Token_Ownerships_V2_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  composed_nfts?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  is_fungible_v2?: InputMaybe<Boolean_Comparison_Exp>;
  is_soulbound_v2?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  property_version_v1?: InputMaybe<Numeric_Comparison_Exp>;
  storage_id?: InputMaybe<String_Comparison_Exp>;
  table_type_v1?: InputMaybe<String_Comparison_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_properties_mutated_v1?: InputMaybe<Jsonb_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
};

/** aggregate max on columns */
export type Current_Token_Ownerships_V2_Max_Fields = {
  __typename?: 'current_token_ownerships_v2_max_fields';
  amount?: Maybe<Scalars['numeric']>;
  last_transaction_timestamp?: Maybe<Scalars['timestamp']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  property_version_v1?: Maybe<Scalars['numeric']>;
  storage_id?: Maybe<Scalars['String']>;
  table_type_v1?: Maybe<Scalars['String']>;
  token_data_id?: Maybe<Scalars['String']>;
  token_standard?: Maybe<Scalars['String']>;
};

/** order by max() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Max_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  storage_id?: InputMaybe<Order_By>;
  table_type_v1?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Current_Token_Ownerships_V2_Min_Fields = {
  __typename?: 'current_token_ownerships_v2_min_fields';
  amount?: Maybe<Scalars['numeric']>;
  last_transaction_timestamp?: Maybe<Scalars['timestamp']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  owner_address?: Maybe<Scalars['String']>;
  property_version_v1?: Maybe<Scalars['numeric']>;
  storage_id?: Maybe<Scalars['String']>;
  table_type_v1?: Maybe<Scalars['String']>;
  token_data_id?: Maybe<Scalars['String']>;
  token_standard?: Maybe<Scalars['String']>;
};

/** order by min() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Min_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  storage_id?: InputMaybe<Order_By>;
  table_type_v1?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "current_token_ownerships_v2". */
export type Current_Token_Ownerships_V2_Order_By = {
  amount?: InputMaybe<Order_By>;
  composed_nfts_aggregate?: InputMaybe<Current_Token_Ownerships_V2_Aggregate_Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Order_By>;
  is_fungible_v2?: InputMaybe<Order_By>;
  is_soulbound_v2?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  storage_id?: InputMaybe<Order_By>;
  table_type_v1?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_properties_mutated_v1?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** select columns of table "current_token_ownerships_v2" */
export enum Current_Token_Ownerships_V2_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  IsFungibleV2 = 'is_fungible_v2',
  /** column name */
  IsSoulboundV2 = 'is_soulbound_v2',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  PropertyVersionV1 = 'property_version_v1',
  /** column name */
  StorageId = 'storage_id',
  /** column name */
  TableTypeV1 = 'table_type_v1',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenPropertiesMutatedV1 = 'token_properties_mutated_v1',
  /** column name */
  TokenStandard = 'token_standard'
}

/** aggregate stddev on columns */
export type Current_Token_Ownerships_V2_Stddev_Fields = {
  __typename?: 'current_token_ownerships_v2_stddev_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
};

/** order by stddev() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Stddev_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** aggregate stddev_pop on columns */
export type Current_Token_Ownerships_V2_Stddev_Pop_Fields = {
  __typename?: 'current_token_ownerships_v2_stddev_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
};

/** order by stddev_pop() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Stddev_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** aggregate stddev_samp on columns */
export type Current_Token_Ownerships_V2_Stddev_Samp_Fields = {
  __typename?: 'current_token_ownerships_v2_stddev_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
};

/** order by stddev_samp() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Stddev_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Token_Ownerships_V2_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Token_Ownerships_V2_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  is_fungible_v2?: InputMaybe<Scalars['Boolean']>;
  is_soulbound_v2?: InputMaybe<Scalars['Boolean']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  owner_address?: InputMaybe<Scalars['String']>;
  property_version_v1?: InputMaybe<Scalars['numeric']>;
  storage_id?: InputMaybe<Scalars['String']>;
  table_type_v1?: InputMaybe<Scalars['String']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_properties_mutated_v1?: InputMaybe<Scalars['jsonb']>;
  token_standard?: InputMaybe<Scalars['String']>;
};

/** aggregate sum on columns */
export type Current_Token_Ownerships_V2_Sum_Fields = {
  __typename?: 'current_token_ownerships_v2_sum_fields';
  amount?: Maybe<Scalars['numeric']>;
  last_transaction_version?: Maybe<Scalars['bigint']>;
  property_version_v1?: Maybe<Scalars['numeric']>;
};

/** order by sum() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Sum_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** aggregate var_pop on columns */
export type Current_Token_Ownerships_V2_Var_Pop_Fields = {
  __typename?: 'current_token_ownerships_v2_var_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
};

/** order by var_pop() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Var_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** aggregate var_samp on columns */
export type Current_Token_Ownerships_V2_Var_Samp_Fields = {
  __typename?: 'current_token_ownerships_v2_var_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
};

/** order by var_samp() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Var_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** aggregate variance on columns */
export type Current_Token_Ownerships_V2_Variance_Fields = {
  __typename?: 'current_token_ownerships_v2_variance_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
};

/** order by variance() on columns of table "current_token_ownerships_v2" */
export type Current_Token_Ownerships_V2_Variance_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
};

/** aggregate var_pop on columns */
export type Current_Token_Ownerships_Var_Pop_Fields = {
  __typename?: 'current_token_ownerships_var_pop_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
};

/** order by var_pop() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Var_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** aggregate var_samp on columns */
export type Current_Token_Ownerships_Var_Samp_Fields = {
  __typename?: 'current_token_ownerships_var_samp_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
};

/** order by var_samp() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Var_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** aggregate variance on columns */
export type Current_Token_Ownerships_Variance_Fields = {
  __typename?: 'current_token_ownerships_variance_fields';
  amount?: Maybe<Scalars['Float']>;
  last_transaction_version?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
};

/** order by variance() on columns of table "current_token_ownerships" */
export type Current_Token_Ownerships_Variance_Order_By = {
  amount?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "current_token_pending_claims" */
export type Current_Token_Pending_Claims = {
  __typename?: 'current_token_pending_claims';
  amount: Scalars['numeric'];
  collection_data_id_hash: Scalars['String'];
  collection_id: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  /** An object relationship */
  current_collection_data?: Maybe<Current_Collection_Datas>;
  /** An object relationship */
  current_collection_v2?: Maybe<Current_Collections_V2>;
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas>;
  /** An object relationship */
  current_token_data_v2?: Maybe<Current_Token_Datas_V2>;
  from_address: Scalars['String'];
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  name: Scalars['String'];
  property_version: Scalars['numeric'];
  table_handle: Scalars['String'];
  to_address: Scalars['String'];
  /** An object relationship */
  token?: Maybe<Tokens>;
  token_data_id: Scalars['String'];
  token_data_id_hash: Scalars['String'];
};

/** Boolean expression to filter rows from the table "current_token_pending_claims". All fields are combined with a logical 'AND'. */
export type Current_Token_Pending_Claims_Bool_Exp = {
  _and?: InputMaybe<Array<Current_Token_Pending_Claims_Bool_Exp>>;
  _not?: InputMaybe<Current_Token_Pending_Claims_Bool_Exp>;
  _or?: InputMaybe<Array<Current_Token_Pending_Claims_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  current_collection_data?: InputMaybe<Current_Collection_Datas_Bool_Exp>;
  current_collection_v2?: InputMaybe<Current_Collections_V2_Bool_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_Bool_Exp>;
  current_token_data_v2?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  from_address?: InputMaybe<String_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  property_version?: InputMaybe<Numeric_Comparison_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
  to_address?: InputMaybe<String_Comparison_Exp>;
  token?: InputMaybe<Tokens_Bool_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_data_id_hash?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "current_token_pending_claims". */
export type Current_Token_Pending_Claims_Order_By = {
  amount?: InputMaybe<Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_id?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  current_collection_data?: InputMaybe<Current_Collection_Datas_Order_By>;
  current_collection_v2?: InputMaybe<Current_Collections_V2_Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_Order_By>;
  current_token_data_v2?: InputMaybe<Current_Token_Datas_V2_Order_By>;
  from_address?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  table_handle?: InputMaybe<Order_By>;
  to_address?: InputMaybe<Order_By>;
  token?: InputMaybe<Tokens_Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
};

/** select columns of table "current_token_pending_claims" */
export enum Current_Token_Pending_Claims_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  FromAddress = 'from_address',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Name = 'name',
  /** column name */
  PropertyVersion = 'property_version',
  /** column name */
  TableHandle = 'table_handle',
  /** column name */
  ToAddress = 'to_address',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenDataIdHash = 'token_data_id_hash'
}

/** Streaming cursor of the table "current_token_pending_claims" */
export type Current_Token_Pending_Claims_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Current_Token_Pending_Claims_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Current_Token_Pending_Claims_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_id?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  from_address?: InputMaybe<Scalars['String']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  name?: InputMaybe<Scalars['String']>;
  property_version?: InputMaybe<Scalars['numeric']>;
  table_handle?: InputMaybe<Scalars['String']>;
  to_address?: InputMaybe<Scalars['String']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_data_id_hash?: InputMaybe<Scalars['String']>;
};

/** ordering argument of a cursor */
export enum Cursor_Ordering {
  /** ascending ordering of the cursor */
  Asc = 'ASC',
  /** descending ordering of the cursor */
  Desc = 'DESC'
}

/** columns and relationships of "delegated_staking_activities" */
export type Delegated_Staking_Activities = {
  __typename?: 'delegated_staking_activities';
  amount: Scalars['numeric'];
  delegator_address: Scalars['String'];
  event_index: Scalars['bigint'];
  event_type: Scalars['String'];
  pool_address: Scalars['String'];
  transaction_version: Scalars['bigint'];
};

/** order by aggregate values of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Aggregate_Order_By = {
  avg?: InputMaybe<Delegated_Staking_Activities_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Delegated_Staking_Activities_Max_Order_By>;
  min?: InputMaybe<Delegated_Staking_Activities_Min_Order_By>;
  stddev?: InputMaybe<Delegated_Staking_Activities_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Delegated_Staking_Activities_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Delegated_Staking_Activities_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Delegated_Staking_Activities_Sum_Order_By>;
  var_pop?: InputMaybe<Delegated_Staking_Activities_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Delegated_Staking_Activities_Var_Samp_Order_By>;
  variance?: InputMaybe<Delegated_Staking_Activities_Variance_Order_By>;
};

/** order by avg() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Avg_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "delegated_staking_activities". All fields are combined with a logical 'AND'. */
export type Delegated_Staking_Activities_Bool_Exp = {
  _and?: InputMaybe<Array<Delegated_Staking_Activities_Bool_Exp>>;
  _not?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
  _or?: InputMaybe<Array<Delegated_Staking_Activities_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  delegator_address?: InputMaybe<String_Comparison_Exp>;
  event_index?: InputMaybe<Bigint_Comparison_Exp>;
  event_type?: InputMaybe<String_Comparison_Exp>;
  pool_address?: InputMaybe<String_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** order by max() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Max_Order_By = {
  amount?: InputMaybe<Order_By>;
  delegator_address?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_type?: InputMaybe<Order_By>;
  pool_address?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by min() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Min_Order_By = {
  amount?: InputMaybe<Order_By>;
  delegator_address?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_type?: InputMaybe<Order_By>;
  pool_address?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "delegated_staking_activities". */
export type Delegated_Staking_Activities_Order_By = {
  amount?: InputMaybe<Order_By>;
  delegator_address?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_type?: InputMaybe<Order_By>;
  pool_address?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "delegated_staking_activities" */
export enum Delegated_Staking_Activities_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  DelegatorAddress = 'delegator_address',
  /** column name */
  EventIndex = 'event_index',
  /** column name */
  EventType = 'event_type',
  /** column name */
  PoolAddress = 'pool_address',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** order by stddev() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Stddev_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by stddev_pop() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Stddev_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by stddev_samp() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Stddev_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Delegated_Staking_Activities_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Delegated_Staking_Activities_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  delegator_address?: InputMaybe<Scalars['String']>;
  event_index?: InputMaybe<Scalars['bigint']>;
  event_type?: InputMaybe<Scalars['String']>;
  pool_address?: InputMaybe<Scalars['String']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** order by sum() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Sum_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by var_pop() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Var_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by var_samp() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Var_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by variance() on columns of table "delegated_staking_activities" */
export type Delegated_Staking_Activities_Variance_Order_By = {
  amount?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "delegated_staking_pools" */
export type Delegated_Staking_Pools = {
  __typename?: 'delegated_staking_pools';
  /** An object relationship */
  current_staking_pool?: Maybe<Current_Staking_Pool_Voter>;
  first_transaction_version: Scalars['bigint'];
  staking_pool_address: Scalars['String'];
};

/** Boolean expression to filter rows from the table "delegated_staking_pools". All fields are combined with a logical 'AND'. */
export type Delegated_Staking_Pools_Bool_Exp = {
  _and?: InputMaybe<Array<Delegated_Staking_Pools_Bool_Exp>>;
  _not?: InputMaybe<Delegated_Staking_Pools_Bool_Exp>;
  _or?: InputMaybe<Array<Delegated_Staking_Pools_Bool_Exp>>;
  current_staking_pool?: InputMaybe<Current_Staking_Pool_Voter_Bool_Exp>;
  first_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  staking_pool_address?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "delegated_staking_pools". */
export type Delegated_Staking_Pools_Order_By = {
  current_staking_pool?: InputMaybe<Current_Staking_Pool_Voter_Order_By>;
  first_transaction_version?: InputMaybe<Order_By>;
  staking_pool_address?: InputMaybe<Order_By>;
};

/** select columns of table "delegated_staking_pools" */
export enum Delegated_Staking_Pools_Select_Column {
  /** column name */
  FirstTransactionVersion = 'first_transaction_version',
  /** column name */
  StakingPoolAddress = 'staking_pool_address'
}

/** Streaming cursor of the table "delegated_staking_pools" */
export type Delegated_Staking_Pools_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Delegated_Staking_Pools_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Delegated_Staking_Pools_Stream_Cursor_Value_Input = {
  first_transaction_version?: InputMaybe<Scalars['bigint']>;
  staking_pool_address?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "delegator_distinct_pool" */
export type Delegator_Distinct_Pool = {
  __typename?: 'delegator_distinct_pool';
  /** An object relationship */
  current_pool_balance?: Maybe<Current_Delegated_Staking_Pool_Balances>;
  delegator_address?: Maybe<Scalars['String']>;
  pool_address?: Maybe<Scalars['String']>;
  /** An object relationship */
  staking_pool_metadata?: Maybe<Current_Staking_Pool_Voter>;
};

/** aggregated selection of "delegator_distinct_pool" */
export type Delegator_Distinct_Pool_Aggregate = {
  __typename?: 'delegator_distinct_pool_aggregate';
  aggregate?: Maybe<Delegator_Distinct_Pool_Aggregate_Fields>;
  nodes: Array<Delegator_Distinct_Pool>;
};

/** aggregate fields of "delegator_distinct_pool" */
export type Delegator_Distinct_Pool_Aggregate_Fields = {
  __typename?: 'delegator_distinct_pool_aggregate_fields';
  count: Scalars['Int'];
  max?: Maybe<Delegator_Distinct_Pool_Max_Fields>;
  min?: Maybe<Delegator_Distinct_Pool_Min_Fields>;
};


/** aggregate fields of "delegator_distinct_pool" */
export type Delegator_Distinct_Pool_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Delegator_Distinct_Pool_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** Boolean expression to filter rows from the table "delegator_distinct_pool". All fields are combined with a logical 'AND'. */
export type Delegator_Distinct_Pool_Bool_Exp = {
  _and?: InputMaybe<Array<Delegator_Distinct_Pool_Bool_Exp>>;
  _not?: InputMaybe<Delegator_Distinct_Pool_Bool_Exp>;
  _or?: InputMaybe<Array<Delegator_Distinct_Pool_Bool_Exp>>;
  current_pool_balance?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Bool_Exp>;
  delegator_address?: InputMaybe<String_Comparison_Exp>;
  pool_address?: InputMaybe<String_Comparison_Exp>;
  staking_pool_metadata?: InputMaybe<Current_Staking_Pool_Voter_Bool_Exp>;
};

/** aggregate max on columns */
export type Delegator_Distinct_Pool_Max_Fields = {
  __typename?: 'delegator_distinct_pool_max_fields';
  delegator_address?: Maybe<Scalars['String']>;
  pool_address?: Maybe<Scalars['String']>;
};

/** aggregate min on columns */
export type Delegator_Distinct_Pool_Min_Fields = {
  __typename?: 'delegator_distinct_pool_min_fields';
  delegator_address?: Maybe<Scalars['String']>;
  pool_address?: Maybe<Scalars['String']>;
};

/** Ordering options when selecting data from "delegator_distinct_pool". */
export type Delegator_Distinct_Pool_Order_By = {
  current_pool_balance?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Order_By>;
  delegator_address?: InputMaybe<Order_By>;
  pool_address?: InputMaybe<Order_By>;
  staking_pool_metadata?: InputMaybe<Current_Staking_Pool_Voter_Order_By>;
};

/** select columns of table "delegator_distinct_pool" */
export enum Delegator_Distinct_Pool_Select_Column {
  /** column name */
  DelegatorAddress = 'delegator_address',
  /** column name */
  PoolAddress = 'pool_address'
}

/** Streaming cursor of the table "delegator_distinct_pool" */
export type Delegator_Distinct_Pool_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Delegator_Distinct_Pool_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Delegator_Distinct_Pool_Stream_Cursor_Value_Input = {
  delegator_address?: InputMaybe<Scalars['String']>;
  pool_address?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "events" */
export type Events = {
  __typename?: 'events';
  account_address: Scalars['String'];
  creation_number: Scalars['bigint'];
  data: Scalars['jsonb'];
  event_index?: Maybe<Scalars['bigint']>;
  sequence_number: Scalars['bigint'];
  transaction_block_height: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
  type: Scalars['String'];
};


/** columns and relationships of "events" */
export type EventsDataArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "events". All fields are combined with a logical 'AND'. */
export type Events_Bool_Exp = {
  _and?: InputMaybe<Array<Events_Bool_Exp>>;
  _not?: InputMaybe<Events_Bool_Exp>;
  _or?: InputMaybe<Array<Events_Bool_Exp>>;
  account_address?: InputMaybe<String_Comparison_Exp>;
  creation_number?: InputMaybe<Bigint_Comparison_Exp>;
  data?: InputMaybe<Jsonb_Comparison_Exp>;
  event_index?: InputMaybe<Bigint_Comparison_Exp>;
  sequence_number?: InputMaybe<Bigint_Comparison_Exp>;
  transaction_block_height?: InputMaybe<Bigint_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "events". */
export type Events_Order_By = {
  account_address?: InputMaybe<Order_By>;
  creation_number?: InputMaybe<Order_By>;
  data?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  sequence_number?: InputMaybe<Order_By>;
  transaction_block_height?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** select columns of table "events" */
export enum Events_Select_Column {
  /** column name */
  AccountAddress = 'account_address',
  /** column name */
  CreationNumber = 'creation_number',
  /** column name */
  Data = 'data',
  /** column name */
  EventIndex = 'event_index',
  /** column name */
  SequenceNumber = 'sequence_number',
  /** column name */
  TransactionBlockHeight = 'transaction_block_height',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  Type = 'type'
}

/** Streaming cursor of the table "events" */
export type Events_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Events_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Events_Stream_Cursor_Value_Input = {
  account_address?: InputMaybe<Scalars['String']>;
  creation_number?: InputMaybe<Scalars['bigint']>;
  data?: InputMaybe<Scalars['jsonb']>;
  event_index?: InputMaybe<Scalars['bigint']>;
  sequence_number?: InputMaybe<Scalars['bigint']>;
  transaction_block_height?: InputMaybe<Scalars['bigint']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  type?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "fungible_asset_activities" */
export type Fungible_Asset_Activities = {
  __typename?: 'fungible_asset_activities';
  amount?: Maybe<Scalars['numeric']>;
  asset_type: Scalars['String'];
  block_height: Scalars['bigint'];
  entry_function_id_str?: Maybe<Scalars['String']>;
  event_index: Scalars['bigint'];
  gas_fee_payer_address?: Maybe<Scalars['String']>;
  is_frozen?: Maybe<Scalars['Boolean']>;
  is_gas_fee: Scalars['Boolean'];
  is_transaction_success: Scalars['Boolean'];
  /** An object relationship */
  metadata?: Maybe<Fungible_Asset_Metadata>;
  owner_address: Scalars['String'];
  /** An array relationship */
  owner_aptos_names: Array<Current_Aptos_Names>;
  storage_id: Scalars['String'];
  token_standard: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
  type: Scalars['String'];
};


/** columns and relationships of "fungible_asset_activities" */
export type Fungible_Asset_ActivitiesOwner_Aptos_NamesArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};

/** order by aggregate values of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Aggregate_Order_By = {
  avg?: InputMaybe<Fungible_Asset_Activities_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Fungible_Asset_Activities_Max_Order_By>;
  min?: InputMaybe<Fungible_Asset_Activities_Min_Order_By>;
  stddev?: InputMaybe<Fungible_Asset_Activities_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Fungible_Asset_Activities_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Fungible_Asset_Activities_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Fungible_Asset_Activities_Sum_Order_By>;
  var_pop?: InputMaybe<Fungible_Asset_Activities_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Fungible_Asset_Activities_Var_Samp_Order_By>;
  variance?: InputMaybe<Fungible_Asset_Activities_Variance_Order_By>;
};

/** order by avg() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Avg_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "fungible_asset_activities". All fields are combined with a logical 'AND'. */
export type Fungible_Asset_Activities_Bool_Exp = {
  _and?: InputMaybe<Array<Fungible_Asset_Activities_Bool_Exp>>;
  _not?: InputMaybe<Fungible_Asset_Activities_Bool_Exp>;
  _or?: InputMaybe<Array<Fungible_Asset_Activities_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  asset_type?: InputMaybe<String_Comparison_Exp>;
  block_height?: InputMaybe<Bigint_Comparison_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  event_index?: InputMaybe<Bigint_Comparison_Exp>;
  gas_fee_payer_address?: InputMaybe<String_Comparison_Exp>;
  is_frozen?: InputMaybe<Boolean_Comparison_Exp>;
  is_gas_fee?: InputMaybe<Boolean_Comparison_Exp>;
  is_transaction_success?: InputMaybe<Boolean_Comparison_Exp>;
  metadata?: InputMaybe<Fungible_Asset_Metadata_Bool_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  owner_aptos_names?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  storage_id?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
};

/** order by max() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Max_Order_By = {
  amount?: InputMaybe<Order_By>;
  asset_type?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  gas_fee_payer_address?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  storage_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** order by min() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Min_Order_By = {
  amount?: InputMaybe<Order_By>;
  asset_type?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  gas_fee_payer_address?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  storage_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "fungible_asset_activities". */
export type Fungible_Asset_Activities_Order_By = {
  amount?: InputMaybe<Order_By>;
  asset_type?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  gas_fee_payer_address?: InputMaybe<Order_By>;
  is_frozen?: InputMaybe<Order_By>;
  is_gas_fee?: InputMaybe<Order_By>;
  is_transaction_success?: InputMaybe<Order_By>;
  metadata?: InputMaybe<Fungible_Asset_Metadata_Order_By>;
  owner_address?: InputMaybe<Order_By>;
  owner_aptos_names_aggregate?: InputMaybe<Current_Aptos_Names_Aggregate_Order_By>;
  storage_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** select columns of table "fungible_asset_activities" */
export enum Fungible_Asset_Activities_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  AssetType = 'asset_type',
  /** column name */
  BlockHeight = 'block_height',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  EventIndex = 'event_index',
  /** column name */
  GasFeePayerAddress = 'gas_fee_payer_address',
  /** column name */
  IsFrozen = 'is_frozen',
  /** column name */
  IsGasFee = 'is_gas_fee',
  /** column name */
  IsTransactionSuccess = 'is_transaction_success',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  StorageId = 'storage_id',
  /** column name */
  TokenStandard = 'token_standard',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  Type = 'type'
}

/** order by stddev() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Stddev_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by stddev_pop() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Stddev_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by stddev_samp() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Stddev_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Fungible_Asset_Activities_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Fungible_Asset_Activities_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  asset_type?: InputMaybe<Scalars['String']>;
  block_height?: InputMaybe<Scalars['bigint']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  event_index?: InputMaybe<Scalars['bigint']>;
  gas_fee_payer_address?: InputMaybe<Scalars['String']>;
  is_frozen?: InputMaybe<Scalars['Boolean']>;
  is_gas_fee?: InputMaybe<Scalars['Boolean']>;
  is_transaction_success?: InputMaybe<Scalars['Boolean']>;
  owner_address?: InputMaybe<Scalars['String']>;
  storage_id?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  type?: InputMaybe<Scalars['String']>;
};

/** order by sum() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Sum_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by var_pop() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Var_Pop_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by var_samp() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Var_Samp_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** order by variance() on columns of table "fungible_asset_activities" */
export type Fungible_Asset_Activities_Variance_Order_By = {
  amount?: InputMaybe<Order_By>;
  block_height?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "fungible_asset_metadata" */
export type Fungible_Asset_Metadata = {
  __typename?: 'fungible_asset_metadata';
  asset_type: Scalars['String'];
  creator_address: Scalars['String'];
  decimals: Scalars['Int'];
  icon_uri?: Maybe<Scalars['String']>;
  last_transaction_timestamp: Scalars['timestamp'];
  last_transaction_version: Scalars['bigint'];
  name: Scalars['String'];
  project_uri?: Maybe<Scalars['String']>;
  supply_aggregator_table_handle_v1?: Maybe<Scalars['String']>;
  supply_aggregator_table_key_v1?: Maybe<Scalars['String']>;
  symbol: Scalars['String'];
  token_standard: Scalars['String'];
};

/** Boolean expression to filter rows from the table "fungible_asset_metadata". All fields are combined with a logical 'AND'. */
export type Fungible_Asset_Metadata_Bool_Exp = {
  _and?: InputMaybe<Array<Fungible_Asset_Metadata_Bool_Exp>>;
  _not?: InputMaybe<Fungible_Asset_Metadata_Bool_Exp>;
  _or?: InputMaybe<Array<Fungible_Asset_Metadata_Bool_Exp>>;
  asset_type?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  decimals?: InputMaybe<Int_Comparison_Exp>;
  icon_uri?: InputMaybe<String_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  project_uri?: InputMaybe<String_Comparison_Exp>;
  supply_aggregator_table_handle_v1?: InputMaybe<String_Comparison_Exp>;
  supply_aggregator_table_key_v1?: InputMaybe<String_Comparison_Exp>;
  symbol?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "fungible_asset_metadata". */
export type Fungible_Asset_Metadata_Order_By = {
  asset_type?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  decimals?: InputMaybe<Order_By>;
  icon_uri?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  project_uri?: InputMaybe<Order_By>;
  supply_aggregator_table_handle_v1?: InputMaybe<Order_By>;
  supply_aggregator_table_key_v1?: InputMaybe<Order_By>;
  symbol?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** select columns of table "fungible_asset_metadata" */
export enum Fungible_Asset_Metadata_Select_Column {
  /** column name */
  AssetType = 'asset_type',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  Decimals = 'decimals',
  /** column name */
  IconUri = 'icon_uri',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Name = 'name',
  /** column name */
  ProjectUri = 'project_uri',
  /** column name */
  SupplyAggregatorTableHandleV1 = 'supply_aggregator_table_handle_v1',
  /** column name */
  SupplyAggregatorTableKeyV1 = 'supply_aggregator_table_key_v1',
  /** column name */
  Symbol = 'symbol',
  /** column name */
  TokenStandard = 'token_standard'
}

/** Streaming cursor of the table "fungible_asset_metadata" */
export type Fungible_Asset_Metadata_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Fungible_Asset_Metadata_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Fungible_Asset_Metadata_Stream_Cursor_Value_Input = {
  asset_type?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  decimals?: InputMaybe<Scalars['Int']>;
  icon_uri?: InputMaybe<Scalars['String']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  name?: InputMaybe<Scalars['String']>;
  project_uri?: InputMaybe<Scalars['String']>;
  supply_aggregator_table_handle_v1?: InputMaybe<Scalars['String']>;
  supply_aggregator_table_key_v1?: InputMaybe<Scalars['String']>;
  symbol?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "indexer_status" */
export type Indexer_Status = {
  __typename?: 'indexer_status';
  db: Scalars['String'];
  is_indexer_up: Scalars['Boolean'];
};

/** Boolean expression to filter rows from the table "indexer_status". All fields are combined with a logical 'AND'. */
export type Indexer_Status_Bool_Exp = {
  _and?: InputMaybe<Array<Indexer_Status_Bool_Exp>>;
  _not?: InputMaybe<Indexer_Status_Bool_Exp>;
  _or?: InputMaybe<Array<Indexer_Status_Bool_Exp>>;
  db?: InputMaybe<String_Comparison_Exp>;
  is_indexer_up?: InputMaybe<Boolean_Comparison_Exp>;
};

/** Ordering options when selecting data from "indexer_status". */
export type Indexer_Status_Order_By = {
  db?: InputMaybe<Order_By>;
  is_indexer_up?: InputMaybe<Order_By>;
};

/** select columns of table "indexer_status" */
export enum Indexer_Status_Select_Column {
  /** column name */
  Db = 'db',
  /** column name */
  IsIndexerUp = 'is_indexer_up'
}

/** Streaming cursor of the table "indexer_status" */
export type Indexer_Status_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Indexer_Status_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Indexer_Status_Stream_Cursor_Value_Input = {
  db?: InputMaybe<Scalars['String']>;
  is_indexer_up?: InputMaybe<Scalars['Boolean']>;
};

export type Jsonb_Cast_Exp = {
  String?: InputMaybe<String_Comparison_Exp>;
};

/** Boolean expression to compare columns of type "jsonb". All fields are combined with logical 'AND'. */
export type Jsonb_Comparison_Exp = {
  _cast?: InputMaybe<Jsonb_Cast_Exp>;
  /** is the column contained in the given json value */
  _contained_in?: InputMaybe<Scalars['jsonb']>;
  /** does the column contain the given json value at the top level */
  _contains?: InputMaybe<Scalars['jsonb']>;
  _eq?: InputMaybe<Scalars['jsonb']>;
  _gt?: InputMaybe<Scalars['jsonb']>;
  _gte?: InputMaybe<Scalars['jsonb']>;
  /** does the string exist as a top-level key in the column */
  _has_key?: InputMaybe<Scalars['String']>;
  /** do all of these strings exist as top-level keys in the column */
  _has_keys_all?: InputMaybe<Array<Scalars['String']>>;
  /** do any of these strings exist as top-level keys in the column */
  _has_keys_any?: InputMaybe<Array<Scalars['String']>>;
  _in?: InputMaybe<Array<Scalars['jsonb']>>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  _lt?: InputMaybe<Scalars['jsonb']>;
  _lte?: InputMaybe<Scalars['jsonb']>;
  _neq?: InputMaybe<Scalars['jsonb']>;
  _nin?: InputMaybe<Array<Scalars['jsonb']>>;
};

/** columns and relationships of "ledger_infos" */
export type Ledger_Infos = {
  __typename?: 'ledger_infos';
  chain_id: Scalars['bigint'];
};

/** Boolean expression to filter rows from the table "ledger_infos". All fields are combined with a logical 'AND'. */
export type Ledger_Infos_Bool_Exp = {
  _and?: InputMaybe<Array<Ledger_Infos_Bool_Exp>>;
  _not?: InputMaybe<Ledger_Infos_Bool_Exp>;
  _or?: InputMaybe<Array<Ledger_Infos_Bool_Exp>>;
  chain_id?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "ledger_infos". */
export type Ledger_Infos_Order_By = {
  chain_id?: InputMaybe<Order_By>;
};

/** select columns of table "ledger_infos" */
export enum Ledger_Infos_Select_Column {
  /** column name */
  ChainId = 'chain_id'
}

/** Streaming cursor of the table "ledger_infos" */
export type Ledger_Infos_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Ledger_Infos_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Ledger_Infos_Stream_Cursor_Value_Input = {
  chain_id?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "move_resources" */
export type Move_Resources = {
  __typename?: 'move_resources';
  address: Scalars['String'];
  transaction_version: Scalars['bigint'];
};

/** aggregated selection of "move_resources" */
export type Move_Resources_Aggregate = {
  __typename?: 'move_resources_aggregate';
  aggregate?: Maybe<Move_Resources_Aggregate_Fields>;
  nodes: Array<Move_Resources>;
};

/** aggregate fields of "move_resources" */
export type Move_Resources_Aggregate_Fields = {
  __typename?: 'move_resources_aggregate_fields';
  avg?: Maybe<Move_Resources_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Move_Resources_Max_Fields>;
  min?: Maybe<Move_Resources_Min_Fields>;
  stddev?: Maybe<Move_Resources_Stddev_Fields>;
  stddev_pop?: Maybe<Move_Resources_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Move_Resources_Stddev_Samp_Fields>;
  sum?: Maybe<Move_Resources_Sum_Fields>;
  var_pop?: Maybe<Move_Resources_Var_Pop_Fields>;
  var_samp?: Maybe<Move_Resources_Var_Samp_Fields>;
  variance?: Maybe<Move_Resources_Variance_Fields>;
};


/** aggregate fields of "move_resources" */
export type Move_Resources_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Move_Resources_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** aggregate avg on columns */
export type Move_Resources_Avg_Fields = {
  __typename?: 'move_resources_avg_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to filter rows from the table "move_resources". All fields are combined with a logical 'AND'. */
export type Move_Resources_Bool_Exp = {
  _and?: InputMaybe<Array<Move_Resources_Bool_Exp>>;
  _not?: InputMaybe<Move_Resources_Bool_Exp>;
  _or?: InputMaybe<Array<Move_Resources_Bool_Exp>>;
  address?: InputMaybe<String_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** aggregate max on columns */
export type Move_Resources_Max_Fields = {
  __typename?: 'move_resources_max_fields';
  address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate min on columns */
export type Move_Resources_Min_Fields = {
  __typename?: 'move_resources_min_fields';
  address?: Maybe<Scalars['String']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** Ordering options when selecting data from "move_resources". */
export type Move_Resources_Order_By = {
  address?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "move_resources" */
export enum Move_Resources_Select_Column {
  /** column name */
  Address = 'address',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** aggregate stddev on columns */
export type Move_Resources_Stddev_Fields = {
  __typename?: 'move_resources_stddev_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_pop on columns */
export type Move_Resources_Stddev_Pop_Fields = {
  __typename?: 'move_resources_stddev_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_samp on columns */
export type Move_Resources_Stddev_Samp_Fields = {
  __typename?: 'move_resources_stddev_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Streaming cursor of the table "move_resources" */
export type Move_Resources_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Move_Resources_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Move_Resources_Stream_Cursor_Value_Input = {
  address?: InputMaybe<Scalars['String']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** aggregate sum on columns */
export type Move_Resources_Sum_Fields = {
  __typename?: 'move_resources_sum_fields';
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate var_pop on columns */
export type Move_Resources_Var_Pop_Fields = {
  __typename?: 'move_resources_var_pop_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate var_samp on columns */
export type Move_Resources_Var_Samp_Fields = {
  __typename?: 'move_resources_var_samp_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate variance on columns */
export type Move_Resources_Variance_Fields = {
  __typename?: 'move_resources_variance_fields';
  transaction_version?: Maybe<Scalars['Float']>;
};

/** columns and relationships of "nft_marketplace_v2.current_nft_marketplace_auctions" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions = {
  __typename?: 'nft_marketplace_v2_current_nft_marketplace_auctions';
  buy_it_now_price?: Maybe<Scalars['numeric']>;
  coin_type?: Maybe<Scalars['String']>;
  collection_id: Scalars['String'];
  contract_address: Scalars['String'];
  current_bid_price?: Maybe<Scalars['numeric']>;
  current_bidder?: Maybe<Scalars['String']>;
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas_V2>;
  entry_function_id_str: Scalars['String'];
  expiration_time: Scalars['numeric'];
  fee_schedule_id: Scalars['String'];
  is_deleted: Scalars['Boolean'];
  last_transaction_timestamp: Scalars['timestamptz'];
  last_transaction_version: Scalars['bigint'];
  listing_id: Scalars['String'];
  marketplace: Scalars['String'];
  seller: Scalars['String'];
  starting_bid_price: Scalars['numeric'];
  token_amount: Scalars['numeric'];
  token_data_id: Scalars['String'];
  token_standard: Scalars['String'];
};

/** Boolean expression to filter rows from the table "nft_marketplace_v2.current_nft_marketplace_auctions". All fields are combined with a logical 'AND'. */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Bool_Exp = {
  _and?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Bool_Exp>>;
  _not?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Bool_Exp>;
  _or?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Bool_Exp>>;
  buy_it_now_price?: InputMaybe<Numeric_Comparison_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  contract_address?: InputMaybe<String_Comparison_Exp>;
  current_bid_price?: InputMaybe<Numeric_Comparison_Exp>;
  current_bidder?: InputMaybe<String_Comparison_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  expiration_time?: InputMaybe<Numeric_Comparison_Exp>;
  fee_schedule_id?: InputMaybe<String_Comparison_Exp>;
  is_deleted?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamptz_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  listing_id?: InputMaybe<String_Comparison_Exp>;
  marketplace?: InputMaybe<String_Comparison_Exp>;
  seller?: InputMaybe<String_Comparison_Exp>;
  starting_bid_price?: InputMaybe<Numeric_Comparison_Exp>;
  token_amount?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "nft_marketplace_v2.current_nft_marketplace_auctions". */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Order_By = {
  buy_it_now_price?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  collection_id?: InputMaybe<Order_By>;
  contract_address?: InputMaybe<Order_By>;
  current_bid_price?: InputMaybe<Order_By>;
  current_bidder?: InputMaybe<Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  expiration_time?: InputMaybe<Order_By>;
  fee_schedule_id?: InputMaybe<Order_By>;
  is_deleted?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  listing_id?: InputMaybe<Order_By>;
  marketplace?: InputMaybe<Order_By>;
  seller?: InputMaybe<Order_By>;
  starting_bid_price?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** select columns of table "nft_marketplace_v2.current_nft_marketplace_auctions" */
export enum Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Select_Column {
  /** column name */
  BuyItNowPrice = 'buy_it_now_price',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  ContractAddress = 'contract_address',
  /** column name */
  CurrentBidPrice = 'current_bid_price',
  /** column name */
  CurrentBidder = 'current_bidder',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  ExpirationTime = 'expiration_time',
  /** column name */
  FeeScheduleId = 'fee_schedule_id',
  /** column name */
  IsDeleted = 'is_deleted',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  ListingId = 'listing_id',
  /** column name */
  Marketplace = 'marketplace',
  /** column name */
  Seller = 'seller',
  /** column name */
  StartingBidPrice = 'starting_bid_price',
  /** column name */
  TokenAmount = 'token_amount',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenStandard = 'token_standard'
}

/** Streaming cursor of the table "nft_marketplace_v2_current_nft_marketplace_auctions" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Stream_Cursor_Value_Input = {
  buy_it_now_price?: InputMaybe<Scalars['numeric']>;
  coin_type?: InputMaybe<Scalars['String']>;
  collection_id?: InputMaybe<Scalars['String']>;
  contract_address?: InputMaybe<Scalars['String']>;
  current_bid_price?: InputMaybe<Scalars['numeric']>;
  current_bidder?: InputMaybe<Scalars['String']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  expiration_time?: InputMaybe<Scalars['numeric']>;
  fee_schedule_id?: InputMaybe<Scalars['String']>;
  is_deleted?: InputMaybe<Scalars['Boolean']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamptz']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  listing_id?: InputMaybe<Scalars['String']>;
  marketplace?: InputMaybe<Scalars['String']>;
  seller?: InputMaybe<Scalars['String']>;
  starting_bid_price?: InputMaybe<Scalars['numeric']>;
  token_amount?: InputMaybe<Scalars['numeric']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "nft_marketplace_v2.current_nft_marketplace_collection_offers" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers = {
  __typename?: 'nft_marketplace_v2_current_nft_marketplace_collection_offers';
  buyer: Scalars['String'];
  coin_type?: Maybe<Scalars['String']>;
  collection_id: Scalars['String'];
  collection_offer_id: Scalars['String'];
  contract_address: Scalars['String'];
  /** An object relationship */
  current_collection_v2?: Maybe<Current_Collections_V2>;
  entry_function_id_str: Scalars['String'];
  expiration_time: Scalars['numeric'];
  fee_schedule_id: Scalars['String'];
  is_deleted: Scalars['Boolean'];
  item_price: Scalars['numeric'];
  last_transaction_timestamp: Scalars['timestamptz'];
  last_transaction_version: Scalars['bigint'];
  marketplace: Scalars['String'];
  remaining_token_amount: Scalars['numeric'];
  token_standard: Scalars['String'];
};

/** Boolean expression to filter rows from the table "nft_marketplace_v2.current_nft_marketplace_collection_offers". All fields are combined with a logical 'AND'. */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Bool_Exp = {
  _and?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Bool_Exp>>;
  _not?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Bool_Exp>;
  _or?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Bool_Exp>>;
  buyer?: InputMaybe<String_Comparison_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  collection_offer_id?: InputMaybe<String_Comparison_Exp>;
  contract_address?: InputMaybe<String_Comparison_Exp>;
  current_collection_v2?: InputMaybe<Current_Collections_V2_Bool_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  expiration_time?: InputMaybe<Numeric_Comparison_Exp>;
  fee_schedule_id?: InputMaybe<String_Comparison_Exp>;
  is_deleted?: InputMaybe<Boolean_Comparison_Exp>;
  item_price?: InputMaybe<Numeric_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamptz_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  marketplace?: InputMaybe<String_Comparison_Exp>;
  remaining_token_amount?: InputMaybe<Numeric_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "nft_marketplace_v2.current_nft_marketplace_collection_offers". */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Order_By = {
  buyer?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  collection_id?: InputMaybe<Order_By>;
  collection_offer_id?: InputMaybe<Order_By>;
  contract_address?: InputMaybe<Order_By>;
  current_collection_v2?: InputMaybe<Current_Collections_V2_Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  expiration_time?: InputMaybe<Order_By>;
  fee_schedule_id?: InputMaybe<Order_By>;
  is_deleted?: InputMaybe<Order_By>;
  item_price?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  marketplace?: InputMaybe<Order_By>;
  remaining_token_amount?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** select columns of table "nft_marketplace_v2.current_nft_marketplace_collection_offers" */
export enum Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Select_Column {
  /** column name */
  Buyer = 'buyer',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  CollectionOfferId = 'collection_offer_id',
  /** column name */
  ContractAddress = 'contract_address',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  ExpirationTime = 'expiration_time',
  /** column name */
  FeeScheduleId = 'fee_schedule_id',
  /** column name */
  IsDeleted = 'is_deleted',
  /** column name */
  ItemPrice = 'item_price',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Marketplace = 'marketplace',
  /** column name */
  RemainingTokenAmount = 'remaining_token_amount',
  /** column name */
  TokenStandard = 'token_standard'
}

/** Streaming cursor of the table "nft_marketplace_v2_current_nft_marketplace_collection_offers" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Stream_Cursor_Value_Input = {
  buyer?: InputMaybe<Scalars['String']>;
  coin_type?: InputMaybe<Scalars['String']>;
  collection_id?: InputMaybe<Scalars['String']>;
  collection_offer_id?: InputMaybe<Scalars['String']>;
  contract_address?: InputMaybe<Scalars['String']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  expiration_time?: InputMaybe<Scalars['numeric']>;
  fee_schedule_id?: InputMaybe<Scalars['String']>;
  is_deleted?: InputMaybe<Scalars['Boolean']>;
  item_price?: InputMaybe<Scalars['numeric']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamptz']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  marketplace?: InputMaybe<Scalars['String']>;
  remaining_token_amount?: InputMaybe<Scalars['numeric']>;
  token_standard?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "nft_marketplace_v2.current_nft_marketplace_listings" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Listings = {
  __typename?: 'nft_marketplace_v2_current_nft_marketplace_listings';
  coin_type?: Maybe<Scalars['String']>;
  collection_id: Scalars['String'];
  contract_address: Scalars['String'];
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas_V2>;
  entry_function_id_str: Scalars['String'];
  fee_schedule_id: Scalars['String'];
  is_deleted: Scalars['Boolean'];
  last_transaction_timestamp: Scalars['timestamptz'];
  last_transaction_version: Scalars['bigint'];
  listing_id: Scalars['String'];
  marketplace: Scalars['String'];
  price: Scalars['numeric'];
  seller: Scalars['String'];
  token_amount: Scalars['numeric'];
  token_data_id: Scalars['String'];
  token_standard: Scalars['String'];
};

/** Boolean expression to filter rows from the table "nft_marketplace_v2.current_nft_marketplace_listings". All fields are combined with a logical 'AND'. */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Bool_Exp = {
  _and?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Bool_Exp>>;
  _not?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Bool_Exp>;
  _or?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Bool_Exp>>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  contract_address?: InputMaybe<String_Comparison_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  fee_schedule_id?: InputMaybe<String_Comparison_Exp>;
  is_deleted?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamptz_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  listing_id?: InputMaybe<String_Comparison_Exp>;
  marketplace?: InputMaybe<String_Comparison_Exp>;
  price?: InputMaybe<Numeric_Comparison_Exp>;
  seller?: InputMaybe<String_Comparison_Exp>;
  token_amount?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "nft_marketplace_v2.current_nft_marketplace_listings". */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Order_By = {
  coin_type?: InputMaybe<Order_By>;
  collection_id?: InputMaybe<Order_By>;
  contract_address?: InputMaybe<Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  fee_schedule_id?: InputMaybe<Order_By>;
  is_deleted?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  listing_id?: InputMaybe<Order_By>;
  marketplace?: InputMaybe<Order_By>;
  price?: InputMaybe<Order_By>;
  seller?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** select columns of table "nft_marketplace_v2.current_nft_marketplace_listings" */
export enum Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Select_Column {
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  ContractAddress = 'contract_address',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  FeeScheduleId = 'fee_schedule_id',
  /** column name */
  IsDeleted = 'is_deleted',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  ListingId = 'listing_id',
  /** column name */
  Marketplace = 'marketplace',
  /** column name */
  Price = 'price',
  /** column name */
  Seller = 'seller',
  /** column name */
  TokenAmount = 'token_amount',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenStandard = 'token_standard'
}

/** Streaming cursor of the table "nft_marketplace_v2_current_nft_marketplace_listings" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Stream_Cursor_Value_Input = {
  coin_type?: InputMaybe<Scalars['String']>;
  collection_id?: InputMaybe<Scalars['String']>;
  contract_address?: InputMaybe<Scalars['String']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  fee_schedule_id?: InputMaybe<Scalars['String']>;
  is_deleted?: InputMaybe<Scalars['Boolean']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamptz']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  listing_id?: InputMaybe<Scalars['String']>;
  marketplace?: InputMaybe<Scalars['String']>;
  price?: InputMaybe<Scalars['numeric']>;
  seller?: InputMaybe<Scalars['String']>;
  token_amount?: InputMaybe<Scalars['numeric']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "nft_marketplace_v2.current_nft_marketplace_token_offers" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers = {
  __typename?: 'nft_marketplace_v2_current_nft_marketplace_token_offers';
  buyer: Scalars['String'];
  coin_type?: Maybe<Scalars['String']>;
  collection_id: Scalars['String'];
  contract_address: Scalars['String'];
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas_V2>;
  entry_function_id_str: Scalars['String'];
  expiration_time: Scalars['numeric'];
  fee_schedule_id: Scalars['String'];
  is_deleted: Scalars['Boolean'];
  last_transaction_timestamp: Scalars['timestamptz'];
  last_transaction_version: Scalars['bigint'];
  marketplace: Scalars['String'];
  offer_id: Scalars['String'];
  price: Scalars['numeric'];
  token_amount: Scalars['numeric'];
  token_data_id: Scalars['String'];
  token_standard: Scalars['String'];
};

/** Boolean expression to filter rows from the table "nft_marketplace_v2.current_nft_marketplace_token_offers". All fields are combined with a logical 'AND'. */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Bool_Exp = {
  _and?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Bool_Exp>>;
  _not?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Bool_Exp>;
  _or?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Bool_Exp>>;
  buyer?: InputMaybe<String_Comparison_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  contract_address?: InputMaybe<String_Comparison_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  expiration_time?: InputMaybe<Numeric_Comparison_Exp>;
  fee_schedule_id?: InputMaybe<String_Comparison_Exp>;
  is_deleted?: InputMaybe<Boolean_Comparison_Exp>;
  last_transaction_timestamp?: InputMaybe<Timestamptz_Comparison_Exp>;
  last_transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  marketplace?: InputMaybe<String_Comparison_Exp>;
  offer_id?: InputMaybe<String_Comparison_Exp>;
  price?: InputMaybe<Numeric_Comparison_Exp>;
  token_amount?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "nft_marketplace_v2.current_nft_marketplace_token_offers". */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Order_By = {
  buyer?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  collection_id?: InputMaybe<Order_By>;
  contract_address?: InputMaybe<Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  expiration_time?: InputMaybe<Order_By>;
  fee_schedule_id?: InputMaybe<Order_By>;
  is_deleted?: InputMaybe<Order_By>;
  last_transaction_timestamp?: InputMaybe<Order_By>;
  last_transaction_version?: InputMaybe<Order_By>;
  marketplace?: InputMaybe<Order_By>;
  offer_id?: InputMaybe<Order_By>;
  price?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
};

/** select columns of table "nft_marketplace_v2.current_nft_marketplace_token_offers" */
export enum Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Select_Column {
  /** column name */
  Buyer = 'buyer',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  ContractAddress = 'contract_address',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  ExpirationTime = 'expiration_time',
  /** column name */
  FeeScheduleId = 'fee_schedule_id',
  /** column name */
  IsDeleted = 'is_deleted',
  /** column name */
  LastTransactionTimestamp = 'last_transaction_timestamp',
  /** column name */
  LastTransactionVersion = 'last_transaction_version',
  /** column name */
  Marketplace = 'marketplace',
  /** column name */
  OfferId = 'offer_id',
  /** column name */
  Price = 'price',
  /** column name */
  TokenAmount = 'token_amount',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenStandard = 'token_standard'
}

/** Streaming cursor of the table "nft_marketplace_v2_current_nft_marketplace_token_offers" */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Stream_Cursor_Value_Input = {
  buyer?: InputMaybe<Scalars['String']>;
  coin_type?: InputMaybe<Scalars['String']>;
  collection_id?: InputMaybe<Scalars['String']>;
  contract_address?: InputMaybe<Scalars['String']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  expiration_time?: InputMaybe<Scalars['numeric']>;
  fee_schedule_id?: InputMaybe<Scalars['String']>;
  is_deleted?: InputMaybe<Scalars['Boolean']>;
  last_transaction_timestamp?: InputMaybe<Scalars['timestamptz']>;
  last_transaction_version?: InputMaybe<Scalars['bigint']>;
  marketplace?: InputMaybe<Scalars['String']>;
  offer_id?: InputMaybe<Scalars['String']>;
  price?: InputMaybe<Scalars['numeric']>;
  token_amount?: InputMaybe<Scalars['numeric']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "nft_marketplace_v2.nft_marketplace_activities" */
export type Nft_Marketplace_V2_Nft_Marketplace_Activities = {
  __typename?: 'nft_marketplace_v2_nft_marketplace_activities';
  buyer?: Maybe<Scalars['String']>;
  coin_type?: Maybe<Scalars['String']>;
  collection_id: Scalars['String'];
  collection_name: Scalars['String'];
  contract_address: Scalars['String'];
  creator_address: Scalars['String'];
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas_V2>;
  entry_function_id_str: Scalars['String'];
  event_index: Scalars['bigint'];
  event_type: Scalars['String'];
  fee_schedule_id: Scalars['String'];
  marketplace: Scalars['String'];
  offer_or_listing_id: Scalars['String'];
  price: Scalars['numeric'];
  property_version?: Maybe<Scalars['String']>;
  seller?: Maybe<Scalars['String']>;
  token_amount: Scalars['numeric'];
  token_data_id?: Maybe<Scalars['String']>;
  token_name?: Maybe<Scalars['String']>;
  token_standard: Scalars['String'];
  transaction_timestamp: Scalars['timestamptz'];
  transaction_version: Scalars['bigint'];
};

/** Boolean expression to filter rows from the table "nft_marketplace_v2.nft_marketplace_activities". All fields are combined with a logical 'AND'. */
export type Nft_Marketplace_V2_Nft_Marketplace_Activities_Bool_Exp = {
  _and?: InputMaybe<Array<Nft_Marketplace_V2_Nft_Marketplace_Activities_Bool_Exp>>;
  _not?: InputMaybe<Nft_Marketplace_V2_Nft_Marketplace_Activities_Bool_Exp>;
  _or?: InputMaybe<Array<Nft_Marketplace_V2_Nft_Marketplace_Activities_Bool_Exp>>;
  buyer?: InputMaybe<String_Comparison_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  collection_id?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  contract_address?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  event_index?: InputMaybe<Bigint_Comparison_Exp>;
  event_type?: InputMaybe<String_Comparison_Exp>;
  fee_schedule_id?: InputMaybe<String_Comparison_Exp>;
  marketplace?: InputMaybe<String_Comparison_Exp>;
  offer_or_listing_id?: InputMaybe<String_Comparison_Exp>;
  price?: InputMaybe<Numeric_Comparison_Exp>;
  property_version?: InputMaybe<String_Comparison_Exp>;
  seller?: InputMaybe<String_Comparison_Exp>;
  token_amount?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_name?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamptz_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "nft_marketplace_v2.nft_marketplace_activities". */
export type Nft_Marketplace_V2_Nft_Marketplace_Activities_Order_By = {
  buyer?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  collection_id?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  contract_address?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_type?: InputMaybe<Order_By>;
  fee_schedule_id?: InputMaybe<Order_By>;
  marketplace?: InputMaybe<Order_By>;
  offer_or_listing_id?: InputMaybe<Order_By>;
  price?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  seller?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_name?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "nft_marketplace_v2.nft_marketplace_activities" */
export enum Nft_Marketplace_V2_Nft_Marketplace_Activities_Select_Column {
  /** column name */
  Buyer = 'buyer',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CollectionId = 'collection_id',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  ContractAddress = 'contract_address',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  EventIndex = 'event_index',
  /** column name */
  EventType = 'event_type',
  /** column name */
  FeeScheduleId = 'fee_schedule_id',
  /** column name */
  Marketplace = 'marketplace',
  /** column name */
  OfferOrListingId = 'offer_or_listing_id',
  /** column name */
  Price = 'price',
  /** column name */
  PropertyVersion = 'property_version',
  /** column name */
  Seller = 'seller',
  /** column name */
  TokenAmount = 'token_amount',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenName = 'token_name',
  /** column name */
  TokenStandard = 'token_standard',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** Streaming cursor of the table "nft_marketplace_v2_nft_marketplace_activities" */
export type Nft_Marketplace_V2_Nft_Marketplace_Activities_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Nft_Marketplace_V2_Nft_Marketplace_Activities_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Nft_Marketplace_V2_Nft_Marketplace_Activities_Stream_Cursor_Value_Input = {
  buyer?: InputMaybe<Scalars['String']>;
  coin_type?: InputMaybe<Scalars['String']>;
  collection_id?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  contract_address?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  event_index?: InputMaybe<Scalars['bigint']>;
  event_type?: InputMaybe<Scalars['String']>;
  fee_schedule_id?: InputMaybe<Scalars['String']>;
  marketplace?: InputMaybe<Scalars['String']>;
  offer_or_listing_id?: InputMaybe<Scalars['String']>;
  price?: InputMaybe<Scalars['numeric']>;
  property_version?: InputMaybe<Scalars['String']>;
  seller?: InputMaybe<Scalars['String']>;
  token_amount?: InputMaybe<Scalars['numeric']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_name?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamptz']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "nft_metadata_crawler.parsed_asset_uris" */
export type Nft_Metadata_Crawler_Parsed_Asset_Uris = {
  __typename?: 'nft_metadata_crawler_parsed_asset_uris';
  asset_uri: Scalars['String'];
  cdn_animation_uri?: Maybe<Scalars['String']>;
  cdn_image_uri?: Maybe<Scalars['String']>;
  cdn_json_uri?: Maybe<Scalars['String']>;
  raw_animation_uri?: Maybe<Scalars['String']>;
  raw_image_uri?: Maybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "nft_metadata_crawler.parsed_asset_uris". All fields are combined with a logical 'AND'. */
export type Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp = {
  _and?: InputMaybe<Array<Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp>>;
  _not?: InputMaybe<Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp>;
  _or?: InputMaybe<Array<Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp>>;
  asset_uri?: InputMaybe<String_Comparison_Exp>;
  cdn_animation_uri?: InputMaybe<String_Comparison_Exp>;
  cdn_image_uri?: InputMaybe<String_Comparison_Exp>;
  cdn_json_uri?: InputMaybe<String_Comparison_Exp>;
  raw_animation_uri?: InputMaybe<String_Comparison_Exp>;
  raw_image_uri?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "nft_metadata_crawler.parsed_asset_uris". */
export type Nft_Metadata_Crawler_Parsed_Asset_Uris_Order_By = {
  asset_uri?: InputMaybe<Order_By>;
  cdn_animation_uri?: InputMaybe<Order_By>;
  cdn_image_uri?: InputMaybe<Order_By>;
  cdn_json_uri?: InputMaybe<Order_By>;
  raw_animation_uri?: InputMaybe<Order_By>;
  raw_image_uri?: InputMaybe<Order_By>;
};

/** select columns of table "nft_metadata_crawler.parsed_asset_uris" */
export enum Nft_Metadata_Crawler_Parsed_Asset_Uris_Select_Column {
  /** column name */
  AssetUri = 'asset_uri',
  /** column name */
  CdnAnimationUri = 'cdn_animation_uri',
  /** column name */
  CdnImageUri = 'cdn_image_uri',
  /** column name */
  CdnJsonUri = 'cdn_json_uri',
  /** column name */
  RawAnimationUri = 'raw_animation_uri',
  /** column name */
  RawImageUri = 'raw_image_uri'
}

/** Streaming cursor of the table "nft_metadata_crawler_parsed_asset_uris" */
export type Nft_Metadata_Crawler_Parsed_Asset_Uris_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Nft_Metadata_Crawler_Parsed_Asset_Uris_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Nft_Metadata_Crawler_Parsed_Asset_Uris_Stream_Cursor_Value_Input = {
  asset_uri?: InputMaybe<Scalars['String']>;
  cdn_animation_uri?: InputMaybe<Scalars['String']>;
  cdn_image_uri?: InputMaybe<Scalars['String']>;
  cdn_json_uri?: InputMaybe<Scalars['String']>;
  raw_animation_uri?: InputMaybe<Scalars['String']>;
  raw_image_uri?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "num_active_delegator_per_pool" */
export type Num_Active_Delegator_Per_Pool = {
  __typename?: 'num_active_delegator_per_pool';
  num_active_delegator?: Maybe<Scalars['bigint']>;
  pool_address?: Maybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "num_active_delegator_per_pool". All fields are combined with a logical 'AND'. */
export type Num_Active_Delegator_Per_Pool_Bool_Exp = {
  _and?: InputMaybe<Array<Num_Active_Delegator_Per_Pool_Bool_Exp>>;
  _not?: InputMaybe<Num_Active_Delegator_Per_Pool_Bool_Exp>;
  _or?: InputMaybe<Array<Num_Active_Delegator_Per_Pool_Bool_Exp>>;
  num_active_delegator?: InputMaybe<Bigint_Comparison_Exp>;
  pool_address?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "num_active_delegator_per_pool". */
export type Num_Active_Delegator_Per_Pool_Order_By = {
  num_active_delegator?: InputMaybe<Order_By>;
  pool_address?: InputMaybe<Order_By>;
};

/** select columns of table "num_active_delegator_per_pool" */
export enum Num_Active_Delegator_Per_Pool_Select_Column {
  /** column name */
  NumActiveDelegator = 'num_active_delegator',
  /** column name */
  PoolAddress = 'pool_address'
}

/** Streaming cursor of the table "num_active_delegator_per_pool" */
export type Num_Active_Delegator_Per_Pool_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Num_Active_Delegator_Per_Pool_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Num_Active_Delegator_Per_Pool_Stream_Cursor_Value_Input = {
  num_active_delegator?: InputMaybe<Scalars['bigint']>;
  pool_address?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to compare columns of type "numeric". All fields are combined with logical 'AND'. */
export type Numeric_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['numeric']>;
  _gt?: InputMaybe<Scalars['numeric']>;
  _gte?: InputMaybe<Scalars['numeric']>;
  _in?: InputMaybe<Array<Scalars['numeric']>>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  _lt?: InputMaybe<Scalars['numeric']>;
  _lte?: InputMaybe<Scalars['numeric']>;
  _neq?: InputMaybe<Scalars['numeric']>;
  _nin?: InputMaybe<Array<Scalars['numeric']>>;
};

/** column ordering options */
export enum Order_By {
  /** in ascending order, nulls last */
  Asc = 'asc',
  /** in ascending order, nulls first */
  AscNullsFirst = 'asc_nulls_first',
  /** in ascending order, nulls last */
  AscNullsLast = 'asc_nulls_last',
  /** in descending order, nulls first */
  Desc = 'desc',
  /** in descending order, nulls first */
  DescNullsFirst = 'desc_nulls_first',
  /** in descending order, nulls last */
  DescNullsLast = 'desc_nulls_last'
}

/** columns and relationships of "processor_status" */
export type Processor_Status = {
  __typename?: 'processor_status';
  last_success_version: Scalars['bigint'];
  last_updated: Scalars['timestamp'];
  processor: Scalars['String'];
};

/** Boolean expression to filter rows from the table "processor_status". All fields are combined with a logical 'AND'. */
export type Processor_Status_Bool_Exp = {
  _and?: InputMaybe<Array<Processor_Status_Bool_Exp>>;
  _not?: InputMaybe<Processor_Status_Bool_Exp>;
  _or?: InputMaybe<Array<Processor_Status_Bool_Exp>>;
  last_success_version?: InputMaybe<Bigint_Comparison_Exp>;
  last_updated?: InputMaybe<Timestamp_Comparison_Exp>;
  processor?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "processor_status". */
export type Processor_Status_Order_By = {
  last_success_version?: InputMaybe<Order_By>;
  last_updated?: InputMaybe<Order_By>;
  processor?: InputMaybe<Order_By>;
};

/** select columns of table "processor_status" */
export enum Processor_Status_Select_Column {
  /** column name */
  LastSuccessVersion = 'last_success_version',
  /** column name */
  LastUpdated = 'last_updated',
  /** column name */
  Processor = 'processor'
}

/** Streaming cursor of the table "processor_status" */
export type Processor_Status_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Processor_Status_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Processor_Status_Stream_Cursor_Value_Input = {
  last_success_version?: InputMaybe<Scalars['bigint']>;
  last_updated?: InputMaybe<Scalars['timestamp']>;
  processor?: InputMaybe<Scalars['String']>;
};

/** columns and relationships of "proposal_votes" */
export type Proposal_Votes = {
  __typename?: 'proposal_votes';
  num_votes: Scalars['numeric'];
  proposal_id: Scalars['bigint'];
  should_pass: Scalars['Boolean'];
  staking_pool_address: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
  voter_address: Scalars['String'];
};

/** aggregated selection of "proposal_votes" */
export type Proposal_Votes_Aggregate = {
  __typename?: 'proposal_votes_aggregate';
  aggregate?: Maybe<Proposal_Votes_Aggregate_Fields>;
  nodes: Array<Proposal_Votes>;
};

/** aggregate fields of "proposal_votes" */
export type Proposal_Votes_Aggregate_Fields = {
  __typename?: 'proposal_votes_aggregate_fields';
  avg?: Maybe<Proposal_Votes_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Proposal_Votes_Max_Fields>;
  min?: Maybe<Proposal_Votes_Min_Fields>;
  stddev?: Maybe<Proposal_Votes_Stddev_Fields>;
  stddev_pop?: Maybe<Proposal_Votes_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Proposal_Votes_Stddev_Samp_Fields>;
  sum?: Maybe<Proposal_Votes_Sum_Fields>;
  var_pop?: Maybe<Proposal_Votes_Var_Pop_Fields>;
  var_samp?: Maybe<Proposal_Votes_Var_Samp_Fields>;
  variance?: Maybe<Proposal_Votes_Variance_Fields>;
};


/** aggregate fields of "proposal_votes" */
export type Proposal_Votes_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Proposal_Votes_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** aggregate avg on columns */
export type Proposal_Votes_Avg_Fields = {
  __typename?: 'proposal_votes_avg_fields';
  num_votes?: Maybe<Scalars['Float']>;
  proposal_id?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Boolean expression to filter rows from the table "proposal_votes". All fields are combined with a logical 'AND'. */
export type Proposal_Votes_Bool_Exp = {
  _and?: InputMaybe<Array<Proposal_Votes_Bool_Exp>>;
  _not?: InputMaybe<Proposal_Votes_Bool_Exp>;
  _or?: InputMaybe<Array<Proposal_Votes_Bool_Exp>>;
  num_votes?: InputMaybe<Numeric_Comparison_Exp>;
  proposal_id?: InputMaybe<Bigint_Comparison_Exp>;
  should_pass?: InputMaybe<Boolean_Comparison_Exp>;
  staking_pool_address?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  voter_address?: InputMaybe<String_Comparison_Exp>;
};

/** aggregate max on columns */
export type Proposal_Votes_Max_Fields = {
  __typename?: 'proposal_votes_max_fields';
  num_votes?: Maybe<Scalars['numeric']>;
  proposal_id?: Maybe<Scalars['bigint']>;
  staking_pool_address?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
  voter_address?: Maybe<Scalars['String']>;
};

/** aggregate min on columns */
export type Proposal_Votes_Min_Fields = {
  __typename?: 'proposal_votes_min_fields';
  num_votes?: Maybe<Scalars['numeric']>;
  proposal_id?: Maybe<Scalars['bigint']>;
  staking_pool_address?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
  voter_address?: Maybe<Scalars['String']>;
};

/** Ordering options when selecting data from "proposal_votes". */
export type Proposal_Votes_Order_By = {
  num_votes?: InputMaybe<Order_By>;
  proposal_id?: InputMaybe<Order_By>;
  should_pass?: InputMaybe<Order_By>;
  staking_pool_address?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  voter_address?: InputMaybe<Order_By>;
};

/** select columns of table "proposal_votes" */
export enum Proposal_Votes_Select_Column {
  /** column name */
  NumVotes = 'num_votes',
  /** column name */
  ProposalId = 'proposal_id',
  /** column name */
  ShouldPass = 'should_pass',
  /** column name */
  StakingPoolAddress = 'staking_pool_address',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  VoterAddress = 'voter_address'
}

/** aggregate stddev on columns */
export type Proposal_Votes_Stddev_Fields = {
  __typename?: 'proposal_votes_stddev_fields';
  num_votes?: Maybe<Scalars['Float']>;
  proposal_id?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_pop on columns */
export type Proposal_Votes_Stddev_Pop_Fields = {
  __typename?: 'proposal_votes_stddev_pop_fields';
  num_votes?: Maybe<Scalars['Float']>;
  proposal_id?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate stddev_samp on columns */
export type Proposal_Votes_Stddev_Samp_Fields = {
  __typename?: 'proposal_votes_stddev_samp_fields';
  num_votes?: Maybe<Scalars['Float']>;
  proposal_id?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** Streaming cursor of the table "proposal_votes" */
export type Proposal_Votes_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Proposal_Votes_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Proposal_Votes_Stream_Cursor_Value_Input = {
  num_votes?: InputMaybe<Scalars['numeric']>;
  proposal_id?: InputMaybe<Scalars['bigint']>;
  should_pass?: InputMaybe<Scalars['Boolean']>;
  staking_pool_address?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  voter_address?: InputMaybe<Scalars['String']>;
};

/** aggregate sum on columns */
export type Proposal_Votes_Sum_Fields = {
  __typename?: 'proposal_votes_sum_fields';
  num_votes?: Maybe<Scalars['numeric']>;
  proposal_id?: Maybe<Scalars['bigint']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** aggregate var_pop on columns */
export type Proposal_Votes_Var_Pop_Fields = {
  __typename?: 'proposal_votes_var_pop_fields';
  num_votes?: Maybe<Scalars['Float']>;
  proposal_id?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate var_samp on columns */
export type Proposal_Votes_Var_Samp_Fields = {
  __typename?: 'proposal_votes_var_samp_fields';
  num_votes?: Maybe<Scalars['Float']>;
  proposal_id?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** aggregate variance on columns */
export type Proposal_Votes_Variance_Fields = {
  __typename?: 'proposal_votes_variance_fields';
  num_votes?: Maybe<Scalars['Float']>;
  proposal_id?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

export type Query_Root = {
  __typename?: 'query_root';
  /** fetch data from the table: "account_transactions" */
  account_transactions: Array<Account_Transactions>;
  /** fetch aggregated fields from the table: "account_transactions" */
  account_transactions_aggregate: Account_Transactions_Aggregate;
  /** fetch data from the table: "account_transactions" using primary key columns */
  account_transactions_by_pk?: Maybe<Account_Transactions>;
  /** fetch data from the table: "address_events_summary" */
  address_events_summary: Array<Address_Events_Summary>;
  /** fetch data from the table: "address_version_from_events" */
  address_version_from_events: Array<Address_Version_From_Events>;
  /** fetch aggregated fields from the table: "address_version_from_events" */
  address_version_from_events_aggregate: Address_Version_From_Events_Aggregate;
  /** fetch data from the table: "address_version_from_move_resources" */
  address_version_from_move_resources: Array<Address_Version_From_Move_Resources>;
  /** fetch aggregated fields from the table: "address_version_from_move_resources" */
  address_version_from_move_resources_aggregate: Address_Version_From_Move_Resources_Aggregate;
  /** fetch data from the table: "block_metadata_transactions" */
  block_metadata_transactions: Array<Block_Metadata_Transactions>;
  /** fetch data from the table: "block_metadata_transactions" using primary key columns */
  block_metadata_transactions_by_pk?: Maybe<Block_Metadata_Transactions>;
  /** An array relationship */
  coin_activities: Array<Coin_Activities>;
  /** An aggregate relationship */
  coin_activities_aggregate: Coin_Activities_Aggregate;
  /** fetch data from the table: "coin_activities" using primary key columns */
  coin_activities_by_pk?: Maybe<Coin_Activities>;
  /** fetch data from the table: "coin_balances" */
  coin_balances: Array<Coin_Balances>;
  /** fetch data from the table: "coin_balances" using primary key columns */
  coin_balances_by_pk?: Maybe<Coin_Balances>;
  /** fetch data from the table: "coin_infos" */
  coin_infos: Array<Coin_Infos>;
  /** fetch data from the table: "coin_infos" using primary key columns */
  coin_infos_by_pk?: Maybe<Coin_Infos>;
  /** fetch data from the table: "coin_supply" */
  coin_supply: Array<Coin_Supply>;
  /** fetch data from the table: "coin_supply" using primary key columns */
  coin_supply_by_pk?: Maybe<Coin_Supply>;
  /** fetch data from the table: "collection_datas" */
  collection_datas: Array<Collection_Datas>;
  /** fetch data from the table: "collection_datas" using primary key columns */
  collection_datas_by_pk?: Maybe<Collection_Datas>;
  /** fetch data from the table: "current_ans_lookup" */
  current_ans_lookup: Array<Current_Ans_Lookup>;
  /** fetch data from the table: "current_ans_lookup" using primary key columns */
  current_ans_lookup_by_pk?: Maybe<Current_Ans_Lookup>;
  /** fetch data from the table: "current_aptos_names" */
  current_aptos_names: Array<Current_Aptos_Names>;
  /** fetch data from the table: "current_coin_balances" */
  current_coin_balances: Array<Current_Coin_Balances>;
  /** fetch data from the table: "current_coin_balances" using primary key columns */
  current_coin_balances_by_pk?: Maybe<Current_Coin_Balances>;
  /** fetch data from the table: "current_collection_datas" */
  current_collection_datas: Array<Current_Collection_Datas>;
  /** fetch data from the table: "current_collection_datas" using primary key columns */
  current_collection_datas_by_pk?: Maybe<Current_Collection_Datas>;
  /** fetch data from the table: "current_collection_ownership_v2_view" */
  current_collection_ownership_v2_view: Array<Current_Collection_Ownership_V2_View>;
  /** fetch aggregated fields from the table: "current_collection_ownership_v2_view" */
  current_collection_ownership_v2_view_aggregate: Current_Collection_Ownership_V2_View_Aggregate;
  /** fetch data from the table: "current_collections_v2" */
  current_collections_v2: Array<Current_Collections_V2>;
  /** fetch data from the table: "current_collections_v2" using primary key columns */
  current_collections_v2_by_pk?: Maybe<Current_Collections_V2>;
  /** fetch data from the table: "current_delegated_staking_pool_balances" */
  current_delegated_staking_pool_balances: Array<Current_Delegated_Staking_Pool_Balances>;
  /** fetch data from the table: "current_delegated_staking_pool_balances" using primary key columns */
  current_delegated_staking_pool_balances_by_pk?: Maybe<Current_Delegated_Staking_Pool_Balances>;
  /** fetch data from the table: "current_delegated_voter" */
  current_delegated_voter: Array<Current_Delegated_Voter>;
  /** fetch data from the table: "current_delegated_voter" using primary key columns */
  current_delegated_voter_by_pk?: Maybe<Current_Delegated_Voter>;
  /** fetch data from the table: "current_delegator_balances" */
  current_delegator_balances: Array<Current_Delegator_Balances>;
  /** fetch data from the table: "current_delegator_balances" using primary key columns */
  current_delegator_balances_by_pk?: Maybe<Current_Delegator_Balances>;
  /** fetch data from the table: "current_fungible_asset_balances" */
  current_fungible_asset_balances: Array<Current_Fungible_Asset_Balances>;
  /** fetch aggregated fields from the table: "current_fungible_asset_balances" */
  current_fungible_asset_balances_aggregate: Current_Fungible_Asset_Balances_Aggregate;
  /** fetch data from the table: "current_fungible_asset_balances" using primary key columns */
  current_fungible_asset_balances_by_pk?: Maybe<Current_Fungible_Asset_Balances>;
  /** fetch data from the table: "current_objects" */
  current_objects: Array<Current_Objects>;
  /** fetch data from the table: "current_objects" using primary key columns */
  current_objects_by_pk?: Maybe<Current_Objects>;
  /** fetch data from the table: "current_staking_pool_voter" */
  current_staking_pool_voter: Array<Current_Staking_Pool_Voter>;
  /** fetch data from the table: "current_staking_pool_voter" using primary key columns */
  current_staking_pool_voter_by_pk?: Maybe<Current_Staking_Pool_Voter>;
  /** fetch data from the table: "current_table_items" */
  current_table_items: Array<Current_Table_Items>;
  /** fetch data from the table: "current_table_items" using primary key columns */
  current_table_items_by_pk?: Maybe<Current_Table_Items>;
  /** fetch data from the table: "current_token_datas" */
  current_token_datas: Array<Current_Token_Datas>;
  /** fetch data from the table: "current_token_datas" using primary key columns */
  current_token_datas_by_pk?: Maybe<Current_Token_Datas>;
  /** fetch data from the table: "current_token_datas_v2" */
  current_token_datas_v2: Array<Current_Token_Datas_V2>;
  /** fetch data from the table: "current_token_datas_v2" using primary key columns */
  current_token_datas_v2_by_pk?: Maybe<Current_Token_Datas_V2>;
  /** fetch data from the table: "current_token_ownerships" */
  current_token_ownerships: Array<Current_Token_Ownerships>;
  /** fetch aggregated fields from the table: "current_token_ownerships" */
  current_token_ownerships_aggregate: Current_Token_Ownerships_Aggregate;
  /** fetch data from the table: "current_token_ownerships" using primary key columns */
  current_token_ownerships_by_pk?: Maybe<Current_Token_Ownerships>;
  /** fetch data from the table: "current_token_ownerships_v2" */
  current_token_ownerships_v2: Array<Current_Token_Ownerships_V2>;
  /** fetch aggregated fields from the table: "current_token_ownerships_v2" */
  current_token_ownerships_v2_aggregate: Current_Token_Ownerships_V2_Aggregate;
  /** fetch data from the table: "current_token_ownerships_v2" using primary key columns */
  current_token_ownerships_v2_by_pk?: Maybe<Current_Token_Ownerships_V2>;
  /** fetch data from the table: "current_token_pending_claims" */
  current_token_pending_claims: Array<Current_Token_Pending_Claims>;
  /** fetch data from the table: "current_token_pending_claims" using primary key columns */
  current_token_pending_claims_by_pk?: Maybe<Current_Token_Pending_Claims>;
  /** An array relationship */
  delegated_staking_activities: Array<Delegated_Staking_Activities>;
  /** fetch data from the table: "delegated_staking_activities" using primary key columns */
  delegated_staking_activities_by_pk?: Maybe<Delegated_Staking_Activities>;
  /** fetch data from the table: "delegated_staking_pools" */
  delegated_staking_pools: Array<Delegated_Staking_Pools>;
  /** fetch data from the table: "delegated_staking_pools" using primary key columns */
  delegated_staking_pools_by_pk?: Maybe<Delegated_Staking_Pools>;
  /** fetch data from the table: "delegator_distinct_pool" */
  delegator_distinct_pool: Array<Delegator_Distinct_Pool>;
  /** fetch aggregated fields from the table: "delegator_distinct_pool" */
  delegator_distinct_pool_aggregate: Delegator_Distinct_Pool_Aggregate;
  /** fetch data from the table: "events" */
  events: Array<Events>;
  /** fetch data from the table: "events" using primary key columns */
  events_by_pk?: Maybe<Events>;
  /** An array relationship */
  fungible_asset_activities: Array<Fungible_Asset_Activities>;
  /** fetch data from the table: "fungible_asset_activities" using primary key columns */
  fungible_asset_activities_by_pk?: Maybe<Fungible_Asset_Activities>;
  /** fetch data from the table: "fungible_asset_metadata" */
  fungible_asset_metadata: Array<Fungible_Asset_Metadata>;
  /** fetch data from the table: "fungible_asset_metadata" using primary key columns */
  fungible_asset_metadata_by_pk?: Maybe<Fungible_Asset_Metadata>;
  /** fetch data from the table: "indexer_status" */
  indexer_status: Array<Indexer_Status>;
  /** fetch data from the table: "indexer_status" using primary key columns */
  indexer_status_by_pk?: Maybe<Indexer_Status>;
  /** fetch data from the table: "ledger_infos" */
  ledger_infos: Array<Ledger_Infos>;
  /** fetch data from the table: "ledger_infos" using primary key columns */
  ledger_infos_by_pk?: Maybe<Ledger_Infos>;
  /** fetch data from the table: "move_resources" */
  move_resources: Array<Move_Resources>;
  /** fetch aggregated fields from the table: "move_resources" */
  move_resources_aggregate: Move_Resources_Aggregate;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_auctions" */
  nft_marketplace_v2_current_nft_marketplace_auctions: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_auctions" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_auctions_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_collection_offers" */
  nft_marketplace_v2_current_nft_marketplace_collection_offers: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_collection_offers" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_collection_offers_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_listings" */
  nft_marketplace_v2_current_nft_marketplace_listings: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_listings" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_listings_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_token_offers" */
  nft_marketplace_v2_current_nft_marketplace_token_offers: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_token_offers" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_token_offers_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.nft_marketplace_activities" */
  nft_marketplace_v2_nft_marketplace_activities: Array<Nft_Marketplace_V2_Nft_Marketplace_Activities>;
  /** fetch data from the table: "nft_marketplace_v2.nft_marketplace_activities" using primary key columns */
  nft_marketplace_v2_nft_marketplace_activities_by_pk?: Maybe<Nft_Marketplace_V2_Nft_Marketplace_Activities>;
  /** fetch data from the table: "nft_metadata_crawler.parsed_asset_uris" */
  nft_metadata_crawler_parsed_asset_uris: Array<Nft_Metadata_Crawler_Parsed_Asset_Uris>;
  /** fetch data from the table: "nft_metadata_crawler.parsed_asset_uris" using primary key columns */
  nft_metadata_crawler_parsed_asset_uris_by_pk?: Maybe<Nft_Metadata_Crawler_Parsed_Asset_Uris>;
  /** fetch data from the table: "num_active_delegator_per_pool" */
  num_active_delegator_per_pool: Array<Num_Active_Delegator_Per_Pool>;
  /** fetch data from the table: "processor_status" */
  processor_status: Array<Processor_Status>;
  /** fetch data from the table: "processor_status" using primary key columns */
  processor_status_by_pk?: Maybe<Processor_Status>;
  /** fetch data from the table: "proposal_votes" */
  proposal_votes: Array<Proposal_Votes>;
  /** fetch aggregated fields from the table: "proposal_votes" */
  proposal_votes_aggregate: Proposal_Votes_Aggregate;
  /** fetch data from the table: "proposal_votes" using primary key columns */
  proposal_votes_by_pk?: Maybe<Proposal_Votes>;
  /** fetch data from the table: "table_items" */
  table_items: Array<Table_Items>;
  /** fetch data from the table: "table_items" using primary key columns */
  table_items_by_pk?: Maybe<Table_Items>;
  /** fetch data from the table: "table_metadatas" */
  table_metadatas: Array<Table_Metadatas>;
  /** fetch data from the table: "table_metadatas" using primary key columns */
  table_metadatas_by_pk?: Maybe<Table_Metadatas>;
  /** An array relationship */
  token_activities: Array<Token_Activities>;
  /** An aggregate relationship */
  token_activities_aggregate: Token_Activities_Aggregate;
  /** fetch data from the table: "token_activities" using primary key columns */
  token_activities_by_pk?: Maybe<Token_Activities>;
  /** An array relationship */
  token_activities_v2: Array<Token_Activities_V2>;
  /** An aggregate relationship */
  token_activities_v2_aggregate: Token_Activities_V2_Aggregate;
  /** fetch data from the table: "token_activities_v2" using primary key columns */
  token_activities_v2_by_pk?: Maybe<Token_Activities_V2>;
  /** fetch data from the table: "token_datas" */
  token_datas: Array<Token_Datas>;
  /** fetch data from the table: "token_datas" using primary key columns */
  token_datas_by_pk?: Maybe<Token_Datas>;
  /** fetch data from the table: "token_ownerships" */
  token_ownerships: Array<Token_Ownerships>;
  /** fetch data from the table: "token_ownerships" using primary key columns */
  token_ownerships_by_pk?: Maybe<Token_Ownerships>;
  /** fetch data from the table: "tokens" */
  tokens: Array<Tokens>;
  /** fetch data from the table: "tokens" using primary key columns */
  tokens_by_pk?: Maybe<Tokens>;
  /** fetch data from the table: "user_transactions" */
  user_transactions: Array<User_Transactions>;
  /** fetch data from the table: "user_transactions" using primary key columns */
  user_transactions_by_pk?: Maybe<User_Transactions>;
};


export type Query_RootAccount_TransactionsArgs = {
  distinct_on?: InputMaybe<Array<Account_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Account_Transactions_Order_By>>;
  where?: InputMaybe<Account_Transactions_Bool_Exp>;
};


export type Query_RootAccount_Transactions_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Account_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Account_Transactions_Order_By>>;
  where?: InputMaybe<Account_Transactions_Bool_Exp>;
};


export type Query_RootAccount_Transactions_By_PkArgs = {
  account_address: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootAddress_Events_SummaryArgs = {
  distinct_on?: InputMaybe<Array<Address_Events_Summary_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Events_Summary_Order_By>>;
  where?: InputMaybe<Address_Events_Summary_Bool_Exp>;
};


export type Query_RootAddress_Version_From_EventsArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Events_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Events_Order_By>>;
  where?: InputMaybe<Address_Version_From_Events_Bool_Exp>;
};


export type Query_RootAddress_Version_From_Events_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Events_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Events_Order_By>>;
  where?: InputMaybe<Address_Version_From_Events_Bool_Exp>;
};


export type Query_RootAddress_Version_From_Move_ResourcesArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Move_Resources_Order_By>>;
  where?: InputMaybe<Address_Version_From_Move_Resources_Bool_Exp>;
};


export type Query_RootAddress_Version_From_Move_Resources_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Move_Resources_Order_By>>;
  where?: InputMaybe<Address_Version_From_Move_Resources_Bool_Exp>;
};


export type Query_RootBlock_Metadata_TransactionsArgs = {
  distinct_on?: InputMaybe<Array<Block_Metadata_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Block_Metadata_Transactions_Order_By>>;
  where?: InputMaybe<Block_Metadata_Transactions_Bool_Exp>;
};


export type Query_RootBlock_Metadata_Transactions_By_PkArgs = {
  version: Scalars['bigint'];
};


export type Query_RootCoin_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


export type Query_RootCoin_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


export type Query_RootCoin_Activities_By_PkArgs = {
  event_account_address: Scalars['String'];
  event_creation_number: Scalars['bigint'];
  event_sequence_number: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootCoin_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Coin_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Balances_Order_By>>;
  where?: InputMaybe<Coin_Balances_Bool_Exp>;
};


export type Query_RootCoin_Balances_By_PkArgs = {
  coin_type_hash: Scalars['String'];
  owner_address: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootCoin_InfosArgs = {
  distinct_on?: InputMaybe<Array<Coin_Infos_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Infos_Order_By>>;
  where?: InputMaybe<Coin_Infos_Bool_Exp>;
};


export type Query_RootCoin_Infos_By_PkArgs = {
  coin_type_hash: Scalars['String'];
};


export type Query_RootCoin_SupplyArgs = {
  distinct_on?: InputMaybe<Array<Coin_Supply_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Supply_Order_By>>;
  where?: InputMaybe<Coin_Supply_Bool_Exp>;
};


export type Query_RootCoin_Supply_By_PkArgs = {
  coin_type_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootCollection_DatasArgs = {
  distinct_on?: InputMaybe<Array<Collection_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Collection_Datas_Order_By>>;
  where?: InputMaybe<Collection_Datas_Bool_Exp>;
};


export type Query_RootCollection_Datas_By_PkArgs = {
  collection_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootCurrent_Ans_LookupArgs = {
  distinct_on?: InputMaybe<Array<Current_Ans_Lookup_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Ans_Lookup_Order_By>>;
  where?: InputMaybe<Current_Ans_Lookup_Bool_Exp>;
};


export type Query_RootCurrent_Ans_Lookup_By_PkArgs = {
  domain: Scalars['String'];
  subdomain: Scalars['String'];
};


export type Query_RootCurrent_Aptos_NamesArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};


export type Query_RootCurrent_Coin_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Coin_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Coin_Balances_Order_By>>;
  where?: InputMaybe<Current_Coin_Balances_Bool_Exp>;
};


export type Query_RootCurrent_Coin_Balances_By_PkArgs = {
  coin_type_hash: Scalars['String'];
  owner_address: Scalars['String'];
};


export type Query_RootCurrent_Collection_DatasArgs = {
  distinct_on?: InputMaybe<Array<Current_Collection_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collection_Datas_Order_By>>;
  where?: InputMaybe<Current_Collection_Datas_Bool_Exp>;
};


export type Query_RootCurrent_Collection_Datas_By_PkArgs = {
  collection_data_id_hash: Scalars['String'];
};


export type Query_RootCurrent_Collection_Ownership_V2_ViewArgs = {
  distinct_on?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Order_By>>;
  where?: InputMaybe<Current_Collection_Ownership_V2_View_Bool_Exp>;
};


export type Query_RootCurrent_Collection_Ownership_V2_View_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Order_By>>;
  where?: InputMaybe<Current_Collection_Ownership_V2_View_Bool_Exp>;
};


export type Query_RootCurrent_Collections_V2Args = {
  distinct_on?: InputMaybe<Array<Current_Collections_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collections_V2_Order_By>>;
  where?: InputMaybe<Current_Collections_V2_Bool_Exp>;
};


export type Query_RootCurrent_Collections_V2_By_PkArgs = {
  collection_id: Scalars['String'];
};


export type Query_RootCurrent_Delegated_Staking_Pool_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Delegated_Staking_Pool_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Delegated_Staking_Pool_Balances_Order_By>>;
  where?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Bool_Exp>;
};


export type Query_RootCurrent_Delegated_Staking_Pool_Balances_By_PkArgs = {
  staking_pool_address: Scalars['String'];
};


export type Query_RootCurrent_Delegated_VoterArgs = {
  distinct_on?: InputMaybe<Array<Current_Delegated_Voter_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Delegated_Voter_Order_By>>;
  where?: InputMaybe<Current_Delegated_Voter_Bool_Exp>;
};


export type Query_RootCurrent_Delegated_Voter_By_PkArgs = {
  delegation_pool_address: Scalars['String'];
  delegator_address: Scalars['String'];
};


export type Query_RootCurrent_Delegator_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Delegator_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Delegator_Balances_Order_By>>;
  where?: InputMaybe<Current_Delegator_Balances_Bool_Exp>;
};


export type Query_RootCurrent_Delegator_Balances_By_PkArgs = {
  delegator_address: Scalars['String'];
  pool_address: Scalars['String'];
  pool_type: Scalars['String'];
  table_handle: Scalars['String'];
};


export type Query_RootCurrent_Fungible_Asset_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Fungible_Asset_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Fungible_Asset_Balances_Order_By>>;
  where?: InputMaybe<Current_Fungible_Asset_Balances_Bool_Exp>;
};


export type Query_RootCurrent_Fungible_Asset_Balances_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Fungible_Asset_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Fungible_Asset_Balances_Order_By>>;
  where?: InputMaybe<Current_Fungible_Asset_Balances_Bool_Exp>;
};


export type Query_RootCurrent_Fungible_Asset_Balances_By_PkArgs = {
  storage_id: Scalars['String'];
};


export type Query_RootCurrent_ObjectsArgs = {
  distinct_on?: InputMaybe<Array<Current_Objects_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Objects_Order_By>>;
  where?: InputMaybe<Current_Objects_Bool_Exp>;
};


export type Query_RootCurrent_Objects_By_PkArgs = {
  object_address: Scalars['String'];
};


export type Query_RootCurrent_Staking_Pool_VoterArgs = {
  distinct_on?: InputMaybe<Array<Current_Staking_Pool_Voter_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Staking_Pool_Voter_Order_By>>;
  where?: InputMaybe<Current_Staking_Pool_Voter_Bool_Exp>;
};


export type Query_RootCurrent_Staking_Pool_Voter_By_PkArgs = {
  staking_pool_address: Scalars['String'];
};


export type Query_RootCurrent_Table_ItemsArgs = {
  distinct_on?: InputMaybe<Array<Current_Table_Items_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Table_Items_Order_By>>;
  where?: InputMaybe<Current_Table_Items_Bool_Exp>;
};


export type Query_RootCurrent_Table_Items_By_PkArgs = {
  key_hash: Scalars['String'];
  table_handle: Scalars['String'];
};


export type Query_RootCurrent_Token_DatasArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Datas_Order_By>>;
  where?: InputMaybe<Current_Token_Datas_Bool_Exp>;
};


export type Query_RootCurrent_Token_Datas_By_PkArgs = {
  token_data_id_hash: Scalars['String'];
};


export type Query_RootCurrent_Token_Datas_V2Args = {
  distinct_on?: InputMaybe<Array<Current_Token_Datas_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Datas_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
};


export type Query_RootCurrent_Token_Datas_V2_By_PkArgs = {
  token_data_id: Scalars['String'];
};


export type Query_RootCurrent_Token_OwnershipsArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
};


export type Query_RootCurrent_Token_Ownerships_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
};


export type Query_RootCurrent_Token_Ownerships_By_PkArgs = {
  owner_address: Scalars['String'];
  property_version: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
};


export type Query_RootCurrent_Token_Ownerships_V2Args = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
};


export type Query_RootCurrent_Token_Ownerships_V2_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
};


export type Query_RootCurrent_Token_Ownerships_V2_By_PkArgs = {
  owner_address: Scalars['String'];
  property_version_v1: Scalars['numeric'];
  storage_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Query_RootCurrent_Token_Pending_ClaimsArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Pending_Claims_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Pending_Claims_Order_By>>;
  where?: InputMaybe<Current_Token_Pending_Claims_Bool_Exp>;
};


export type Query_RootCurrent_Token_Pending_Claims_By_PkArgs = {
  from_address: Scalars['String'];
  property_version: Scalars['numeric'];
  to_address: Scalars['String'];
  token_data_id_hash: Scalars['String'];
};


export type Query_RootDelegated_Staking_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Delegated_Staking_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegated_Staking_Activities_Order_By>>;
  where?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
};


export type Query_RootDelegated_Staking_Activities_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootDelegated_Staking_PoolsArgs = {
  distinct_on?: InputMaybe<Array<Delegated_Staking_Pools_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegated_Staking_Pools_Order_By>>;
  where?: InputMaybe<Delegated_Staking_Pools_Bool_Exp>;
};


export type Query_RootDelegated_Staking_Pools_By_PkArgs = {
  staking_pool_address: Scalars['String'];
};


export type Query_RootDelegator_Distinct_PoolArgs = {
  distinct_on?: InputMaybe<Array<Delegator_Distinct_Pool_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegator_Distinct_Pool_Order_By>>;
  where?: InputMaybe<Delegator_Distinct_Pool_Bool_Exp>;
};


export type Query_RootDelegator_Distinct_Pool_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Delegator_Distinct_Pool_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegator_Distinct_Pool_Order_By>>;
  where?: InputMaybe<Delegator_Distinct_Pool_Bool_Exp>;
};


export type Query_RootEventsArgs = {
  distinct_on?: InputMaybe<Array<Events_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Events_Order_By>>;
  where?: InputMaybe<Events_Bool_Exp>;
};


export type Query_RootEvents_By_PkArgs = {
  account_address: Scalars['String'];
  creation_number: Scalars['bigint'];
  sequence_number: Scalars['bigint'];
};


export type Query_RootFungible_Asset_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Fungible_Asset_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Fungible_Asset_Activities_Order_By>>;
  where?: InputMaybe<Fungible_Asset_Activities_Bool_Exp>;
};


export type Query_RootFungible_Asset_Activities_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootFungible_Asset_MetadataArgs = {
  distinct_on?: InputMaybe<Array<Fungible_Asset_Metadata_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Fungible_Asset_Metadata_Order_By>>;
  where?: InputMaybe<Fungible_Asset_Metadata_Bool_Exp>;
};


export type Query_RootFungible_Asset_Metadata_By_PkArgs = {
  asset_type: Scalars['String'];
};


export type Query_RootIndexer_StatusArgs = {
  distinct_on?: InputMaybe<Array<Indexer_Status_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Indexer_Status_Order_By>>;
  where?: InputMaybe<Indexer_Status_Bool_Exp>;
};


export type Query_RootIndexer_Status_By_PkArgs = {
  db: Scalars['String'];
};


export type Query_RootLedger_InfosArgs = {
  distinct_on?: InputMaybe<Array<Ledger_Infos_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Ledger_Infos_Order_By>>;
  where?: InputMaybe<Ledger_Infos_Bool_Exp>;
};


export type Query_RootLedger_Infos_By_PkArgs = {
  chain_id: Scalars['bigint'];
};


export type Query_RootMove_ResourcesArgs = {
  distinct_on?: InputMaybe<Array<Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Move_Resources_Order_By>>;
  where?: InputMaybe<Move_Resources_Bool_Exp>;
};


export type Query_RootMove_Resources_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Move_Resources_Order_By>>;
  where?: InputMaybe<Move_Resources_Bool_Exp>;
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_AuctionsArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Bool_Exp>;
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_Auctions_By_PkArgs = {
  listing_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_Collection_OffersArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Bool_Exp>;
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_By_PkArgs = {
  collection_id: Scalars['String'];
  collection_offer_id: Scalars['String'];
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_ListingsArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Bool_Exp>;
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_Listings_By_PkArgs = {
  listing_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_Token_OffersArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Bool_Exp>;
};


export type Query_RootNft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_By_PkArgs = {
  offer_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Query_RootNft_Marketplace_V2_Nft_Marketplace_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Nft_Marketplace_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Nft_Marketplace_Activities_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Nft_Marketplace_Activities_Bool_Exp>;
};


export type Query_RootNft_Marketplace_V2_Nft_Marketplace_Activities_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootNft_Metadata_Crawler_Parsed_Asset_UrisArgs = {
  distinct_on?: InputMaybe<Array<Nft_Metadata_Crawler_Parsed_Asset_Uris_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Metadata_Crawler_Parsed_Asset_Uris_Order_By>>;
  where?: InputMaybe<Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp>;
};


export type Query_RootNft_Metadata_Crawler_Parsed_Asset_Uris_By_PkArgs = {
  asset_uri: Scalars['String'];
};


export type Query_RootNum_Active_Delegator_Per_PoolArgs = {
  distinct_on?: InputMaybe<Array<Num_Active_Delegator_Per_Pool_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Num_Active_Delegator_Per_Pool_Order_By>>;
  where?: InputMaybe<Num_Active_Delegator_Per_Pool_Bool_Exp>;
};


export type Query_RootProcessor_StatusArgs = {
  distinct_on?: InputMaybe<Array<Processor_Status_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Processor_Status_Order_By>>;
  where?: InputMaybe<Processor_Status_Bool_Exp>;
};


export type Query_RootProcessor_Status_By_PkArgs = {
  processor: Scalars['String'];
};


export type Query_RootProposal_VotesArgs = {
  distinct_on?: InputMaybe<Array<Proposal_Votes_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Proposal_Votes_Order_By>>;
  where?: InputMaybe<Proposal_Votes_Bool_Exp>;
};


export type Query_RootProposal_Votes_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Proposal_Votes_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Proposal_Votes_Order_By>>;
  where?: InputMaybe<Proposal_Votes_Bool_Exp>;
};


export type Query_RootProposal_Votes_By_PkArgs = {
  proposal_id: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
  voter_address: Scalars['String'];
};


export type Query_RootTable_ItemsArgs = {
  distinct_on?: InputMaybe<Array<Table_Items_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Table_Items_Order_By>>;
  where?: InputMaybe<Table_Items_Bool_Exp>;
};


export type Query_RootTable_Items_By_PkArgs = {
  transaction_version: Scalars['bigint'];
  write_set_change_index: Scalars['bigint'];
};


export type Query_RootTable_MetadatasArgs = {
  distinct_on?: InputMaybe<Array<Table_Metadatas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Table_Metadatas_Order_By>>;
  where?: InputMaybe<Table_Metadatas_Bool_Exp>;
};


export type Query_RootTable_Metadatas_By_PkArgs = {
  handle: Scalars['String'];
};


export type Query_RootToken_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


export type Query_RootToken_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


export type Query_RootToken_Activities_By_PkArgs = {
  event_account_address: Scalars['String'];
  event_creation_number: Scalars['bigint'];
  event_sequence_number: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootToken_Activities_V2Args = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


export type Query_RootToken_Activities_V2_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


export type Query_RootToken_Activities_V2_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootToken_DatasArgs = {
  distinct_on?: InputMaybe<Array<Token_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Datas_Order_By>>;
  where?: InputMaybe<Token_Datas_Bool_Exp>;
};


export type Query_RootToken_Datas_By_PkArgs = {
  token_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootToken_OwnershipsArgs = {
  distinct_on?: InputMaybe<Array<Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Ownerships_Order_By>>;
  where?: InputMaybe<Token_Ownerships_Bool_Exp>;
};


export type Query_RootToken_Ownerships_By_PkArgs = {
  property_version: Scalars['numeric'];
  table_handle: Scalars['String'];
  token_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootTokensArgs = {
  distinct_on?: InputMaybe<Array<Tokens_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Tokens_Order_By>>;
  where?: InputMaybe<Tokens_Bool_Exp>;
};


export type Query_RootTokens_By_PkArgs = {
  property_version: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Query_RootUser_TransactionsArgs = {
  distinct_on?: InputMaybe<Array<User_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<User_Transactions_Order_By>>;
  where?: InputMaybe<User_Transactions_Bool_Exp>;
};


export type Query_RootUser_Transactions_By_PkArgs = {
  version: Scalars['bigint'];
};

export type Subscription_Root = {
  __typename?: 'subscription_root';
  /** fetch data from the table: "account_transactions" */
  account_transactions: Array<Account_Transactions>;
  /** fetch aggregated fields from the table: "account_transactions" */
  account_transactions_aggregate: Account_Transactions_Aggregate;
  /** fetch data from the table: "account_transactions" using primary key columns */
  account_transactions_by_pk?: Maybe<Account_Transactions>;
  /** fetch data from the table in a streaming manner : "account_transactions" */
  account_transactions_stream: Array<Account_Transactions>;
  /** fetch data from the table: "address_events_summary" */
  address_events_summary: Array<Address_Events_Summary>;
  /** fetch data from the table in a streaming manner : "address_events_summary" */
  address_events_summary_stream: Array<Address_Events_Summary>;
  /** fetch data from the table: "address_version_from_events" */
  address_version_from_events: Array<Address_Version_From_Events>;
  /** fetch aggregated fields from the table: "address_version_from_events" */
  address_version_from_events_aggregate: Address_Version_From_Events_Aggregate;
  /** fetch data from the table in a streaming manner : "address_version_from_events" */
  address_version_from_events_stream: Array<Address_Version_From_Events>;
  /** fetch data from the table: "address_version_from_move_resources" */
  address_version_from_move_resources: Array<Address_Version_From_Move_Resources>;
  /** fetch aggregated fields from the table: "address_version_from_move_resources" */
  address_version_from_move_resources_aggregate: Address_Version_From_Move_Resources_Aggregate;
  /** fetch data from the table in a streaming manner : "address_version_from_move_resources" */
  address_version_from_move_resources_stream: Array<Address_Version_From_Move_Resources>;
  /** fetch data from the table: "block_metadata_transactions" */
  block_metadata_transactions: Array<Block_Metadata_Transactions>;
  /** fetch data from the table: "block_metadata_transactions" using primary key columns */
  block_metadata_transactions_by_pk?: Maybe<Block_Metadata_Transactions>;
  /** fetch data from the table in a streaming manner : "block_metadata_transactions" */
  block_metadata_transactions_stream: Array<Block_Metadata_Transactions>;
  /** An array relationship */
  coin_activities: Array<Coin_Activities>;
  /** An aggregate relationship */
  coin_activities_aggregate: Coin_Activities_Aggregate;
  /** fetch data from the table: "coin_activities" using primary key columns */
  coin_activities_by_pk?: Maybe<Coin_Activities>;
  /** fetch data from the table in a streaming manner : "coin_activities" */
  coin_activities_stream: Array<Coin_Activities>;
  /** fetch data from the table: "coin_balances" */
  coin_balances: Array<Coin_Balances>;
  /** fetch data from the table: "coin_balances" using primary key columns */
  coin_balances_by_pk?: Maybe<Coin_Balances>;
  /** fetch data from the table in a streaming manner : "coin_balances" */
  coin_balances_stream: Array<Coin_Balances>;
  /** fetch data from the table: "coin_infos" */
  coin_infos: Array<Coin_Infos>;
  /** fetch data from the table: "coin_infos" using primary key columns */
  coin_infos_by_pk?: Maybe<Coin_Infos>;
  /** fetch data from the table in a streaming manner : "coin_infos" */
  coin_infos_stream: Array<Coin_Infos>;
  /** fetch data from the table: "coin_supply" */
  coin_supply: Array<Coin_Supply>;
  /** fetch data from the table: "coin_supply" using primary key columns */
  coin_supply_by_pk?: Maybe<Coin_Supply>;
  /** fetch data from the table in a streaming manner : "coin_supply" */
  coin_supply_stream: Array<Coin_Supply>;
  /** fetch data from the table: "collection_datas" */
  collection_datas: Array<Collection_Datas>;
  /** fetch data from the table: "collection_datas" using primary key columns */
  collection_datas_by_pk?: Maybe<Collection_Datas>;
  /** fetch data from the table in a streaming manner : "collection_datas" */
  collection_datas_stream: Array<Collection_Datas>;
  /** fetch data from the table: "current_ans_lookup" */
  current_ans_lookup: Array<Current_Ans_Lookup>;
  /** fetch data from the table: "current_ans_lookup" using primary key columns */
  current_ans_lookup_by_pk?: Maybe<Current_Ans_Lookup>;
  /** fetch data from the table in a streaming manner : "current_ans_lookup" */
  current_ans_lookup_stream: Array<Current_Ans_Lookup>;
  /** fetch data from the table: "current_aptos_names" */
  current_aptos_names: Array<Current_Aptos_Names>;
  /** fetch data from the table in a streaming manner : "current_aptos_names" */
  current_aptos_names_stream: Array<Current_Aptos_Names>;
  /** fetch data from the table: "current_coin_balances" */
  current_coin_balances: Array<Current_Coin_Balances>;
  /** fetch data from the table: "current_coin_balances" using primary key columns */
  current_coin_balances_by_pk?: Maybe<Current_Coin_Balances>;
  /** fetch data from the table in a streaming manner : "current_coin_balances" */
  current_coin_balances_stream: Array<Current_Coin_Balances>;
  /** fetch data from the table: "current_collection_datas" */
  current_collection_datas: Array<Current_Collection_Datas>;
  /** fetch data from the table: "current_collection_datas" using primary key columns */
  current_collection_datas_by_pk?: Maybe<Current_Collection_Datas>;
  /** fetch data from the table in a streaming manner : "current_collection_datas" */
  current_collection_datas_stream: Array<Current_Collection_Datas>;
  /** fetch data from the table: "current_collection_ownership_v2_view" */
  current_collection_ownership_v2_view: Array<Current_Collection_Ownership_V2_View>;
  /** fetch aggregated fields from the table: "current_collection_ownership_v2_view" */
  current_collection_ownership_v2_view_aggregate: Current_Collection_Ownership_V2_View_Aggregate;
  /** fetch data from the table in a streaming manner : "current_collection_ownership_v2_view" */
  current_collection_ownership_v2_view_stream: Array<Current_Collection_Ownership_V2_View>;
  /** fetch data from the table: "current_collections_v2" */
  current_collections_v2: Array<Current_Collections_V2>;
  /** fetch data from the table: "current_collections_v2" using primary key columns */
  current_collections_v2_by_pk?: Maybe<Current_Collections_V2>;
  /** fetch data from the table in a streaming manner : "current_collections_v2" */
  current_collections_v2_stream: Array<Current_Collections_V2>;
  /** fetch data from the table: "current_delegated_staking_pool_balances" */
  current_delegated_staking_pool_balances: Array<Current_Delegated_Staking_Pool_Balances>;
  /** fetch data from the table: "current_delegated_staking_pool_balances" using primary key columns */
  current_delegated_staking_pool_balances_by_pk?: Maybe<Current_Delegated_Staking_Pool_Balances>;
  /** fetch data from the table in a streaming manner : "current_delegated_staking_pool_balances" */
  current_delegated_staking_pool_balances_stream: Array<Current_Delegated_Staking_Pool_Balances>;
  /** fetch data from the table: "current_delegated_voter" */
  current_delegated_voter: Array<Current_Delegated_Voter>;
  /** fetch data from the table: "current_delegated_voter" using primary key columns */
  current_delegated_voter_by_pk?: Maybe<Current_Delegated_Voter>;
  /** fetch data from the table in a streaming manner : "current_delegated_voter" */
  current_delegated_voter_stream: Array<Current_Delegated_Voter>;
  /** fetch data from the table: "current_delegator_balances" */
  current_delegator_balances: Array<Current_Delegator_Balances>;
  /** fetch data from the table: "current_delegator_balances" using primary key columns */
  current_delegator_balances_by_pk?: Maybe<Current_Delegator_Balances>;
  /** fetch data from the table in a streaming manner : "current_delegator_balances" */
  current_delegator_balances_stream: Array<Current_Delegator_Balances>;
  /** fetch data from the table: "current_fungible_asset_balances" */
  current_fungible_asset_balances: Array<Current_Fungible_Asset_Balances>;
  /** fetch aggregated fields from the table: "current_fungible_asset_balances" */
  current_fungible_asset_balances_aggregate: Current_Fungible_Asset_Balances_Aggregate;
  /** fetch data from the table: "current_fungible_asset_balances" using primary key columns */
  current_fungible_asset_balances_by_pk?: Maybe<Current_Fungible_Asset_Balances>;
  /** fetch data from the table in a streaming manner : "current_fungible_asset_balances" */
  current_fungible_asset_balances_stream: Array<Current_Fungible_Asset_Balances>;
  /** fetch data from the table: "current_objects" */
  current_objects: Array<Current_Objects>;
  /** fetch data from the table: "current_objects" using primary key columns */
  current_objects_by_pk?: Maybe<Current_Objects>;
  /** fetch data from the table in a streaming manner : "current_objects" */
  current_objects_stream: Array<Current_Objects>;
  /** fetch data from the table: "current_staking_pool_voter" */
  current_staking_pool_voter: Array<Current_Staking_Pool_Voter>;
  /** fetch data from the table: "current_staking_pool_voter" using primary key columns */
  current_staking_pool_voter_by_pk?: Maybe<Current_Staking_Pool_Voter>;
  /** fetch data from the table in a streaming manner : "current_staking_pool_voter" */
  current_staking_pool_voter_stream: Array<Current_Staking_Pool_Voter>;
  /** fetch data from the table: "current_table_items" */
  current_table_items: Array<Current_Table_Items>;
  /** fetch data from the table: "current_table_items" using primary key columns */
  current_table_items_by_pk?: Maybe<Current_Table_Items>;
  /** fetch data from the table in a streaming manner : "current_table_items" */
  current_table_items_stream: Array<Current_Table_Items>;
  /** fetch data from the table: "current_token_datas" */
  current_token_datas: Array<Current_Token_Datas>;
  /** fetch data from the table: "current_token_datas" using primary key columns */
  current_token_datas_by_pk?: Maybe<Current_Token_Datas>;
  /** fetch data from the table in a streaming manner : "current_token_datas" */
  current_token_datas_stream: Array<Current_Token_Datas>;
  /** fetch data from the table: "current_token_datas_v2" */
  current_token_datas_v2: Array<Current_Token_Datas_V2>;
  /** fetch data from the table: "current_token_datas_v2" using primary key columns */
  current_token_datas_v2_by_pk?: Maybe<Current_Token_Datas_V2>;
  /** fetch data from the table in a streaming manner : "current_token_datas_v2" */
  current_token_datas_v2_stream: Array<Current_Token_Datas_V2>;
  /** fetch data from the table: "current_token_ownerships" */
  current_token_ownerships: Array<Current_Token_Ownerships>;
  /** fetch aggregated fields from the table: "current_token_ownerships" */
  current_token_ownerships_aggregate: Current_Token_Ownerships_Aggregate;
  /** fetch data from the table: "current_token_ownerships" using primary key columns */
  current_token_ownerships_by_pk?: Maybe<Current_Token_Ownerships>;
  /** fetch data from the table in a streaming manner : "current_token_ownerships" */
  current_token_ownerships_stream: Array<Current_Token_Ownerships>;
  /** fetch data from the table: "current_token_ownerships_v2" */
  current_token_ownerships_v2: Array<Current_Token_Ownerships_V2>;
  /** fetch aggregated fields from the table: "current_token_ownerships_v2" */
  current_token_ownerships_v2_aggregate: Current_Token_Ownerships_V2_Aggregate;
  /** fetch data from the table: "current_token_ownerships_v2" using primary key columns */
  current_token_ownerships_v2_by_pk?: Maybe<Current_Token_Ownerships_V2>;
  /** fetch data from the table in a streaming manner : "current_token_ownerships_v2" */
  current_token_ownerships_v2_stream: Array<Current_Token_Ownerships_V2>;
  /** fetch data from the table: "current_token_pending_claims" */
  current_token_pending_claims: Array<Current_Token_Pending_Claims>;
  /** fetch data from the table: "current_token_pending_claims" using primary key columns */
  current_token_pending_claims_by_pk?: Maybe<Current_Token_Pending_Claims>;
  /** fetch data from the table in a streaming manner : "current_token_pending_claims" */
  current_token_pending_claims_stream: Array<Current_Token_Pending_Claims>;
  /** An array relationship */
  delegated_staking_activities: Array<Delegated_Staking_Activities>;
  /** fetch data from the table: "delegated_staking_activities" using primary key columns */
  delegated_staking_activities_by_pk?: Maybe<Delegated_Staking_Activities>;
  /** fetch data from the table in a streaming manner : "delegated_staking_activities" */
  delegated_staking_activities_stream: Array<Delegated_Staking_Activities>;
  /** fetch data from the table: "delegated_staking_pools" */
  delegated_staking_pools: Array<Delegated_Staking_Pools>;
  /** fetch data from the table: "delegated_staking_pools" using primary key columns */
  delegated_staking_pools_by_pk?: Maybe<Delegated_Staking_Pools>;
  /** fetch data from the table in a streaming manner : "delegated_staking_pools" */
  delegated_staking_pools_stream: Array<Delegated_Staking_Pools>;
  /** fetch data from the table: "delegator_distinct_pool" */
  delegator_distinct_pool: Array<Delegator_Distinct_Pool>;
  /** fetch aggregated fields from the table: "delegator_distinct_pool" */
  delegator_distinct_pool_aggregate: Delegator_Distinct_Pool_Aggregate;
  /** fetch data from the table in a streaming manner : "delegator_distinct_pool" */
  delegator_distinct_pool_stream: Array<Delegator_Distinct_Pool>;
  /** fetch data from the table: "events" */
  events: Array<Events>;
  /** fetch data from the table: "events" using primary key columns */
  events_by_pk?: Maybe<Events>;
  /** fetch data from the table in a streaming manner : "events" */
  events_stream: Array<Events>;
  /** An array relationship */
  fungible_asset_activities: Array<Fungible_Asset_Activities>;
  /** fetch data from the table: "fungible_asset_activities" using primary key columns */
  fungible_asset_activities_by_pk?: Maybe<Fungible_Asset_Activities>;
  /** fetch data from the table in a streaming manner : "fungible_asset_activities" */
  fungible_asset_activities_stream: Array<Fungible_Asset_Activities>;
  /** fetch data from the table: "fungible_asset_metadata" */
  fungible_asset_metadata: Array<Fungible_Asset_Metadata>;
  /** fetch data from the table: "fungible_asset_metadata" using primary key columns */
  fungible_asset_metadata_by_pk?: Maybe<Fungible_Asset_Metadata>;
  /** fetch data from the table in a streaming manner : "fungible_asset_metadata" */
  fungible_asset_metadata_stream: Array<Fungible_Asset_Metadata>;
  /** fetch data from the table: "indexer_status" */
  indexer_status: Array<Indexer_Status>;
  /** fetch data from the table: "indexer_status" using primary key columns */
  indexer_status_by_pk?: Maybe<Indexer_Status>;
  /** fetch data from the table in a streaming manner : "indexer_status" */
  indexer_status_stream: Array<Indexer_Status>;
  /** fetch data from the table: "ledger_infos" */
  ledger_infos: Array<Ledger_Infos>;
  /** fetch data from the table: "ledger_infos" using primary key columns */
  ledger_infos_by_pk?: Maybe<Ledger_Infos>;
  /** fetch data from the table in a streaming manner : "ledger_infos" */
  ledger_infos_stream: Array<Ledger_Infos>;
  /** fetch data from the table: "move_resources" */
  move_resources: Array<Move_Resources>;
  /** fetch aggregated fields from the table: "move_resources" */
  move_resources_aggregate: Move_Resources_Aggregate;
  /** fetch data from the table in a streaming manner : "move_resources" */
  move_resources_stream: Array<Move_Resources>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_auctions" */
  nft_marketplace_v2_current_nft_marketplace_auctions: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_auctions" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_auctions_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions>;
  /** fetch data from the table in a streaming manner : "nft_marketplace_v2.current_nft_marketplace_auctions" */
  nft_marketplace_v2_current_nft_marketplace_auctions_stream: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_collection_offers" */
  nft_marketplace_v2_current_nft_marketplace_collection_offers: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_collection_offers" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_collection_offers_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers>;
  /** fetch data from the table in a streaming manner : "nft_marketplace_v2.current_nft_marketplace_collection_offers" */
  nft_marketplace_v2_current_nft_marketplace_collection_offers_stream: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_listings" */
  nft_marketplace_v2_current_nft_marketplace_listings: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_listings" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_listings_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings>;
  /** fetch data from the table in a streaming manner : "nft_marketplace_v2.current_nft_marketplace_listings" */
  nft_marketplace_v2_current_nft_marketplace_listings_stream: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_token_offers" */
  nft_marketplace_v2_current_nft_marketplace_token_offers: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.current_nft_marketplace_token_offers" using primary key columns */
  nft_marketplace_v2_current_nft_marketplace_token_offers_by_pk?: Maybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers>;
  /** fetch data from the table in a streaming manner : "nft_marketplace_v2.current_nft_marketplace_token_offers" */
  nft_marketplace_v2_current_nft_marketplace_token_offers_stream: Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers>;
  /** fetch data from the table: "nft_marketplace_v2.nft_marketplace_activities" */
  nft_marketplace_v2_nft_marketplace_activities: Array<Nft_Marketplace_V2_Nft_Marketplace_Activities>;
  /** fetch data from the table: "nft_marketplace_v2.nft_marketplace_activities" using primary key columns */
  nft_marketplace_v2_nft_marketplace_activities_by_pk?: Maybe<Nft_Marketplace_V2_Nft_Marketplace_Activities>;
  /** fetch data from the table in a streaming manner : "nft_marketplace_v2.nft_marketplace_activities" */
  nft_marketplace_v2_nft_marketplace_activities_stream: Array<Nft_Marketplace_V2_Nft_Marketplace_Activities>;
  /** fetch data from the table: "nft_metadata_crawler.parsed_asset_uris" */
  nft_metadata_crawler_parsed_asset_uris: Array<Nft_Metadata_Crawler_Parsed_Asset_Uris>;
  /** fetch data from the table: "nft_metadata_crawler.parsed_asset_uris" using primary key columns */
  nft_metadata_crawler_parsed_asset_uris_by_pk?: Maybe<Nft_Metadata_Crawler_Parsed_Asset_Uris>;
  /** fetch data from the table in a streaming manner : "nft_metadata_crawler.parsed_asset_uris" */
  nft_metadata_crawler_parsed_asset_uris_stream: Array<Nft_Metadata_Crawler_Parsed_Asset_Uris>;
  /** fetch data from the table: "num_active_delegator_per_pool" */
  num_active_delegator_per_pool: Array<Num_Active_Delegator_Per_Pool>;
  /** fetch data from the table in a streaming manner : "num_active_delegator_per_pool" */
  num_active_delegator_per_pool_stream: Array<Num_Active_Delegator_Per_Pool>;
  /** fetch data from the table: "processor_status" */
  processor_status: Array<Processor_Status>;
  /** fetch data from the table: "processor_status" using primary key columns */
  processor_status_by_pk?: Maybe<Processor_Status>;
  /** fetch data from the table in a streaming manner : "processor_status" */
  processor_status_stream: Array<Processor_Status>;
  /** fetch data from the table: "proposal_votes" */
  proposal_votes: Array<Proposal_Votes>;
  /** fetch aggregated fields from the table: "proposal_votes" */
  proposal_votes_aggregate: Proposal_Votes_Aggregate;
  /** fetch data from the table: "proposal_votes" using primary key columns */
  proposal_votes_by_pk?: Maybe<Proposal_Votes>;
  /** fetch data from the table in a streaming manner : "proposal_votes" */
  proposal_votes_stream: Array<Proposal_Votes>;
  /** fetch data from the table: "table_items" */
  table_items: Array<Table_Items>;
  /** fetch data from the table: "table_items" using primary key columns */
  table_items_by_pk?: Maybe<Table_Items>;
  /** fetch data from the table in a streaming manner : "table_items" */
  table_items_stream: Array<Table_Items>;
  /** fetch data from the table: "table_metadatas" */
  table_metadatas: Array<Table_Metadatas>;
  /** fetch data from the table: "table_metadatas" using primary key columns */
  table_metadatas_by_pk?: Maybe<Table_Metadatas>;
  /** fetch data from the table in a streaming manner : "table_metadatas" */
  table_metadatas_stream: Array<Table_Metadatas>;
  /** An array relationship */
  token_activities: Array<Token_Activities>;
  /** An aggregate relationship */
  token_activities_aggregate: Token_Activities_Aggregate;
  /** fetch data from the table: "token_activities" using primary key columns */
  token_activities_by_pk?: Maybe<Token_Activities>;
  /** fetch data from the table in a streaming manner : "token_activities" */
  token_activities_stream: Array<Token_Activities>;
  /** An array relationship */
  token_activities_v2: Array<Token_Activities_V2>;
  /** An aggregate relationship */
  token_activities_v2_aggregate: Token_Activities_V2_Aggregate;
  /** fetch data from the table: "token_activities_v2" using primary key columns */
  token_activities_v2_by_pk?: Maybe<Token_Activities_V2>;
  /** fetch data from the table in a streaming manner : "token_activities_v2" */
  token_activities_v2_stream: Array<Token_Activities_V2>;
  /** fetch data from the table: "token_datas" */
  token_datas: Array<Token_Datas>;
  /** fetch data from the table: "token_datas" using primary key columns */
  token_datas_by_pk?: Maybe<Token_Datas>;
  /** fetch data from the table in a streaming manner : "token_datas" */
  token_datas_stream: Array<Token_Datas>;
  /** fetch data from the table: "token_ownerships" */
  token_ownerships: Array<Token_Ownerships>;
  /** fetch data from the table: "token_ownerships" using primary key columns */
  token_ownerships_by_pk?: Maybe<Token_Ownerships>;
  /** fetch data from the table in a streaming manner : "token_ownerships" */
  token_ownerships_stream: Array<Token_Ownerships>;
  /** fetch data from the table: "tokens" */
  tokens: Array<Tokens>;
  /** fetch data from the table: "tokens" using primary key columns */
  tokens_by_pk?: Maybe<Tokens>;
  /** fetch data from the table in a streaming manner : "tokens" */
  tokens_stream: Array<Tokens>;
  /** fetch data from the table: "user_transactions" */
  user_transactions: Array<User_Transactions>;
  /** fetch data from the table: "user_transactions" using primary key columns */
  user_transactions_by_pk?: Maybe<User_Transactions>;
  /** fetch data from the table in a streaming manner : "user_transactions" */
  user_transactions_stream: Array<User_Transactions>;
};


export type Subscription_RootAccount_TransactionsArgs = {
  distinct_on?: InputMaybe<Array<Account_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Account_Transactions_Order_By>>;
  where?: InputMaybe<Account_Transactions_Bool_Exp>;
};


export type Subscription_RootAccount_Transactions_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Account_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Account_Transactions_Order_By>>;
  where?: InputMaybe<Account_Transactions_Bool_Exp>;
};


export type Subscription_RootAccount_Transactions_By_PkArgs = {
  account_address: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootAccount_Transactions_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Account_Transactions_Stream_Cursor_Input>>;
  where?: InputMaybe<Account_Transactions_Bool_Exp>;
};


export type Subscription_RootAddress_Events_SummaryArgs = {
  distinct_on?: InputMaybe<Array<Address_Events_Summary_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Events_Summary_Order_By>>;
  where?: InputMaybe<Address_Events_Summary_Bool_Exp>;
};


export type Subscription_RootAddress_Events_Summary_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Address_Events_Summary_Stream_Cursor_Input>>;
  where?: InputMaybe<Address_Events_Summary_Bool_Exp>;
};


export type Subscription_RootAddress_Version_From_EventsArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Events_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Events_Order_By>>;
  where?: InputMaybe<Address_Version_From_Events_Bool_Exp>;
};


export type Subscription_RootAddress_Version_From_Events_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Events_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Events_Order_By>>;
  where?: InputMaybe<Address_Version_From_Events_Bool_Exp>;
};


export type Subscription_RootAddress_Version_From_Events_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Address_Version_From_Events_Stream_Cursor_Input>>;
  where?: InputMaybe<Address_Version_From_Events_Bool_Exp>;
};


export type Subscription_RootAddress_Version_From_Move_ResourcesArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Move_Resources_Order_By>>;
  where?: InputMaybe<Address_Version_From_Move_Resources_Bool_Exp>;
};


export type Subscription_RootAddress_Version_From_Move_Resources_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Address_Version_From_Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Address_Version_From_Move_Resources_Order_By>>;
  where?: InputMaybe<Address_Version_From_Move_Resources_Bool_Exp>;
};


export type Subscription_RootAddress_Version_From_Move_Resources_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Address_Version_From_Move_Resources_Stream_Cursor_Input>>;
  where?: InputMaybe<Address_Version_From_Move_Resources_Bool_Exp>;
};


export type Subscription_RootBlock_Metadata_TransactionsArgs = {
  distinct_on?: InputMaybe<Array<Block_Metadata_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Block_Metadata_Transactions_Order_By>>;
  where?: InputMaybe<Block_Metadata_Transactions_Bool_Exp>;
};


export type Subscription_RootBlock_Metadata_Transactions_By_PkArgs = {
  version: Scalars['bigint'];
};


export type Subscription_RootBlock_Metadata_Transactions_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Block_Metadata_Transactions_Stream_Cursor_Input>>;
  where?: InputMaybe<Block_Metadata_Transactions_Bool_Exp>;
};


export type Subscription_RootCoin_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


export type Subscription_RootCoin_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Coin_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Activities_Order_By>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


export type Subscription_RootCoin_Activities_By_PkArgs = {
  event_account_address: Scalars['String'];
  event_creation_number: Scalars['bigint'];
  event_sequence_number: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootCoin_Activities_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Coin_Activities_Stream_Cursor_Input>>;
  where?: InputMaybe<Coin_Activities_Bool_Exp>;
};


export type Subscription_RootCoin_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Coin_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Balances_Order_By>>;
  where?: InputMaybe<Coin_Balances_Bool_Exp>;
};


export type Subscription_RootCoin_Balances_By_PkArgs = {
  coin_type_hash: Scalars['String'];
  owner_address: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootCoin_Balances_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Coin_Balances_Stream_Cursor_Input>>;
  where?: InputMaybe<Coin_Balances_Bool_Exp>;
};


export type Subscription_RootCoin_InfosArgs = {
  distinct_on?: InputMaybe<Array<Coin_Infos_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Infos_Order_By>>;
  where?: InputMaybe<Coin_Infos_Bool_Exp>;
};


export type Subscription_RootCoin_Infos_By_PkArgs = {
  coin_type_hash: Scalars['String'];
};


export type Subscription_RootCoin_Infos_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Coin_Infos_Stream_Cursor_Input>>;
  where?: InputMaybe<Coin_Infos_Bool_Exp>;
};


export type Subscription_RootCoin_SupplyArgs = {
  distinct_on?: InputMaybe<Array<Coin_Supply_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Coin_Supply_Order_By>>;
  where?: InputMaybe<Coin_Supply_Bool_Exp>;
};


export type Subscription_RootCoin_Supply_By_PkArgs = {
  coin_type_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootCoin_Supply_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Coin_Supply_Stream_Cursor_Input>>;
  where?: InputMaybe<Coin_Supply_Bool_Exp>;
};


export type Subscription_RootCollection_DatasArgs = {
  distinct_on?: InputMaybe<Array<Collection_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Collection_Datas_Order_By>>;
  where?: InputMaybe<Collection_Datas_Bool_Exp>;
};


export type Subscription_RootCollection_Datas_By_PkArgs = {
  collection_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootCollection_Datas_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Collection_Datas_Stream_Cursor_Input>>;
  where?: InputMaybe<Collection_Datas_Bool_Exp>;
};


export type Subscription_RootCurrent_Ans_LookupArgs = {
  distinct_on?: InputMaybe<Array<Current_Ans_Lookup_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Ans_Lookup_Order_By>>;
  where?: InputMaybe<Current_Ans_Lookup_Bool_Exp>;
};


export type Subscription_RootCurrent_Ans_Lookup_By_PkArgs = {
  domain: Scalars['String'];
  subdomain: Scalars['String'];
};


export type Subscription_RootCurrent_Ans_Lookup_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Ans_Lookup_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Ans_Lookup_Bool_Exp>;
};


export type Subscription_RootCurrent_Aptos_NamesArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};


export type Subscription_RootCurrent_Aptos_Names_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Aptos_Names_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};


export type Subscription_RootCurrent_Coin_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Coin_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Coin_Balances_Order_By>>;
  where?: InputMaybe<Current_Coin_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Coin_Balances_By_PkArgs = {
  coin_type_hash: Scalars['String'];
  owner_address: Scalars['String'];
};


export type Subscription_RootCurrent_Coin_Balances_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Coin_Balances_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Coin_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Collection_DatasArgs = {
  distinct_on?: InputMaybe<Array<Current_Collection_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collection_Datas_Order_By>>;
  where?: InputMaybe<Current_Collection_Datas_Bool_Exp>;
};


export type Subscription_RootCurrent_Collection_Datas_By_PkArgs = {
  collection_data_id_hash: Scalars['String'];
};


export type Subscription_RootCurrent_Collection_Datas_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Collection_Datas_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Collection_Datas_Bool_Exp>;
};


export type Subscription_RootCurrent_Collection_Ownership_V2_ViewArgs = {
  distinct_on?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Order_By>>;
  where?: InputMaybe<Current_Collection_Ownership_V2_View_Bool_Exp>;
};


export type Subscription_RootCurrent_Collection_Ownership_V2_View_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collection_Ownership_V2_View_Order_By>>;
  where?: InputMaybe<Current_Collection_Ownership_V2_View_Bool_Exp>;
};


export type Subscription_RootCurrent_Collection_Ownership_V2_View_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Collection_Ownership_V2_View_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Collection_Ownership_V2_View_Bool_Exp>;
};


export type Subscription_RootCurrent_Collections_V2Args = {
  distinct_on?: InputMaybe<Array<Current_Collections_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Collections_V2_Order_By>>;
  where?: InputMaybe<Current_Collections_V2_Bool_Exp>;
};


export type Subscription_RootCurrent_Collections_V2_By_PkArgs = {
  collection_id: Scalars['String'];
};


export type Subscription_RootCurrent_Collections_V2_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Collections_V2_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Collections_V2_Bool_Exp>;
};


export type Subscription_RootCurrent_Delegated_Staking_Pool_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Delegated_Staking_Pool_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Delegated_Staking_Pool_Balances_Order_By>>;
  where?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Delegated_Staking_Pool_Balances_By_PkArgs = {
  staking_pool_address: Scalars['String'];
};


export type Subscription_RootCurrent_Delegated_Staking_Pool_Balances_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Delegated_Staking_Pool_Balances_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Delegated_Staking_Pool_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Delegated_VoterArgs = {
  distinct_on?: InputMaybe<Array<Current_Delegated_Voter_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Delegated_Voter_Order_By>>;
  where?: InputMaybe<Current_Delegated_Voter_Bool_Exp>;
};


export type Subscription_RootCurrent_Delegated_Voter_By_PkArgs = {
  delegation_pool_address: Scalars['String'];
  delegator_address: Scalars['String'];
};


export type Subscription_RootCurrent_Delegated_Voter_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Delegated_Voter_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Delegated_Voter_Bool_Exp>;
};


export type Subscription_RootCurrent_Delegator_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Delegator_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Delegator_Balances_Order_By>>;
  where?: InputMaybe<Current_Delegator_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Delegator_Balances_By_PkArgs = {
  delegator_address: Scalars['String'];
  pool_address: Scalars['String'];
  pool_type: Scalars['String'];
  table_handle: Scalars['String'];
};


export type Subscription_RootCurrent_Delegator_Balances_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Delegator_Balances_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Delegator_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Fungible_Asset_BalancesArgs = {
  distinct_on?: InputMaybe<Array<Current_Fungible_Asset_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Fungible_Asset_Balances_Order_By>>;
  where?: InputMaybe<Current_Fungible_Asset_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Fungible_Asset_Balances_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Fungible_Asset_Balances_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Fungible_Asset_Balances_Order_By>>;
  where?: InputMaybe<Current_Fungible_Asset_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_Fungible_Asset_Balances_By_PkArgs = {
  storage_id: Scalars['String'];
};


export type Subscription_RootCurrent_Fungible_Asset_Balances_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Fungible_Asset_Balances_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Fungible_Asset_Balances_Bool_Exp>;
};


export type Subscription_RootCurrent_ObjectsArgs = {
  distinct_on?: InputMaybe<Array<Current_Objects_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Objects_Order_By>>;
  where?: InputMaybe<Current_Objects_Bool_Exp>;
};


export type Subscription_RootCurrent_Objects_By_PkArgs = {
  object_address: Scalars['String'];
};


export type Subscription_RootCurrent_Objects_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Objects_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Objects_Bool_Exp>;
};


export type Subscription_RootCurrent_Staking_Pool_VoterArgs = {
  distinct_on?: InputMaybe<Array<Current_Staking_Pool_Voter_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Staking_Pool_Voter_Order_By>>;
  where?: InputMaybe<Current_Staking_Pool_Voter_Bool_Exp>;
};


export type Subscription_RootCurrent_Staking_Pool_Voter_By_PkArgs = {
  staking_pool_address: Scalars['String'];
};


export type Subscription_RootCurrent_Staking_Pool_Voter_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Staking_Pool_Voter_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Staking_Pool_Voter_Bool_Exp>;
};


export type Subscription_RootCurrent_Table_ItemsArgs = {
  distinct_on?: InputMaybe<Array<Current_Table_Items_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Table_Items_Order_By>>;
  where?: InputMaybe<Current_Table_Items_Bool_Exp>;
};


export type Subscription_RootCurrent_Table_Items_By_PkArgs = {
  key_hash: Scalars['String'];
  table_handle: Scalars['String'];
};


export type Subscription_RootCurrent_Table_Items_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Table_Items_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Table_Items_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_DatasArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Datas_Order_By>>;
  where?: InputMaybe<Current_Token_Datas_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Datas_By_PkArgs = {
  token_data_id_hash: Scalars['String'];
};


export type Subscription_RootCurrent_Token_Datas_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Token_Datas_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Token_Datas_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Datas_V2Args = {
  distinct_on?: InputMaybe<Array<Current_Token_Datas_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Datas_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Datas_V2_By_PkArgs = {
  token_data_id: Scalars['String'];
};


export type Subscription_RootCurrent_Token_Datas_V2_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Token_Datas_V2_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_OwnershipsArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Ownerships_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Ownerships_By_PkArgs = {
  owner_address: Scalars['String'];
  property_version: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
};


export type Subscription_RootCurrent_Token_Ownerships_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Token_Ownerships_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Token_Ownerships_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Ownerships_V2Args = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Ownerships_V2_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Ownerships_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Ownerships_V2_Order_By>>;
  where?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Ownerships_V2_By_PkArgs = {
  owner_address: Scalars['String'];
  property_version_v1: Scalars['numeric'];
  storage_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Subscription_RootCurrent_Token_Ownerships_V2_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Token_Ownerships_V2_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Token_Ownerships_V2_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Pending_ClaimsArgs = {
  distinct_on?: InputMaybe<Array<Current_Token_Pending_Claims_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Token_Pending_Claims_Order_By>>;
  where?: InputMaybe<Current_Token_Pending_Claims_Bool_Exp>;
};


export type Subscription_RootCurrent_Token_Pending_Claims_By_PkArgs = {
  from_address: Scalars['String'];
  property_version: Scalars['numeric'];
  to_address: Scalars['String'];
  token_data_id_hash: Scalars['String'];
};


export type Subscription_RootCurrent_Token_Pending_Claims_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Current_Token_Pending_Claims_Stream_Cursor_Input>>;
  where?: InputMaybe<Current_Token_Pending_Claims_Bool_Exp>;
};


export type Subscription_RootDelegated_Staking_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Delegated_Staking_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegated_Staking_Activities_Order_By>>;
  where?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
};


export type Subscription_RootDelegated_Staking_Activities_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootDelegated_Staking_Activities_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Delegated_Staking_Activities_Stream_Cursor_Input>>;
  where?: InputMaybe<Delegated_Staking_Activities_Bool_Exp>;
};


export type Subscription_RootDelegated_Staking_PoolsArgs = {
  distinct_on?: InputMaybe<Array<Delegated_Staking_Pools_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegated_Staking_Pools_Order_By>>;
  where?: InputMaybe<Delegated_Staking_Pools_Bool_Exp>;
};


export type Subscription_RootDelegated_Staking_Pools_By_PkArgs = {
  staking_pool_address: Scalars['String'];
};


export type Subscription_RootDelegated_Staking_Pools_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Delegated_Staking_Pools_Stream_Cursor_Input>>;
  where?: InputMaybe<Delegated_Staking_Pools_Bool_Exp>;
};


export type Subscription_RootDelegator_Distinct_PoolArgs = {
  distinct_on?: InputMaybe<Array<Delegator_Distinct_Pool_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegator_Distinct_Pool_Order_By>>;
  where?: InputMaybe<Delegator_Distinct_Pool_Bool_Exp>;
};


export type Subscription_RootDelegator_Distinct_Pool_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Delegator_Distinct_Pool_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Delegator_Distinct_Pool_Order_By>>;
  where?: InputMaybe<Delegator_Distinct_Pool_Bool_Exp>;
};


export type Subscription_RootDelegator_Distinct_Pool_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Delegator_Distinct_Pool_Stream_Cursor_Input>>;
  where?: InputMaybe<Delegator_Distinct_Pool_Bool_Exp>;
};


export type Subscription_RootEventsArgs = {
  distinct_on?: InputMaybe<Array<Events_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Events_Order_By>>;
  where?: InputMaybe<Events_Bool_Exp>;
};


export type Subscription_RootEvents_By_PkArgs = {
  account_address: Scalars['String'];
  creation_number: Scalars['bigint'];
  sequence_number: Scalars['bigint'];
};


export type Subscription_RootEvents_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Events_Stream_Cursor_Input>>;
  where?: InputMaybe<Events_Bool_Exp>;
};


export type Subscription_RootFungible_Asset_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Fungible_Asset_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Fungible_Asset_Activities_Order_By>>;
  where?: InputMaybe<Fungible_Asset_Activities_Bool_Exp>;
};


export type Subscription_RootFungible_Asset_Activities_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootFungible_Asset_Activities_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Fungible_Asset_Activities_Stream_Cursor_Input>>;
  where?: InputMaybe<Fungible_Asset_Activities_Bool_Exp>;
};


export type Subscription_RootFungible_Asset_MetadataArgs = {
  distinct_on?: InputMaybe<Array<Fungible_Asset_Metadata_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Fungible_Asset_Metadata_Order_By>>;
  where?: InputMaybe<Fungible_Asset_Metadata_Bool_Exp>;
};


export type Subscription_RootFungible_Asset_Metadata_By_PkArgs = {
  asset_type: Scalars['String'];
};


export type Subscription_RootFungible_Asset_Metadata_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Fungible_Asset_Metadata_Stream_Cursor_Input>>;
  where?: InputMaybe<Fungible_Asset_Metadata_Bool_Exp>;
};


export type Subscription_RootIndexer_StatusArgs = {
  distinct_on?: InputMaybe<Array<Indexer_Status_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Indexer_Status_Order_By>>;
  where?: InputMaybe<Indexer_Status_Bool_Exp>;
};


export type Subscription_RootIndexer_Status_By_PkArgs = {
  db: Scalars['String'];
};


export type Subscription_RootIndexer_Status_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Indexer_Status_Stream_Cursor_Input>>;
  where?: InputMaybe<Indexer_Status_Bool_Exp>;
};


export type Subscription_RootLedger_InfosArgs = {
  distinct_on?: InputMaybe<Array<Ledger_Infos_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Ledger_Infos_Order_By>>;
  where?: InputMaybe<Ledger_Infos_Bool_Exp>;
};


export type Subscription_RootLedger_Infos_By_PkArgs = {
  chain_id: Scalars['bigint'];
};


export type Subscription_RootLedger_Infos_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Ledger_Infos_Stream_Cursor_Input>>;
  where?: InputMaybe<Ledger_Infos_Bool_Exp>;
};


export type Subscription_RootMove_ResourcesArgs = {
  distinct_on?: InputMaybe<Array<Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Move_Resources_Order_By>>;
  where?: InputMaybe<Move_Resources_Bool_Exp>;
};


export type Subscription_RootMove_Resources_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Move_Resources_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Move_Resources_Order_By>>;
  where?: InputMaybe<Move_Resources_Bool_Exp>;
};


export type Subscription_RootMove_Resources_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Move_Resources_Stream_Cursor_Input>>;
  where?: InputMaybe<Move_Resources_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_AuctionsArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Auctions_By_PkArgs = {
  listing_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Auctions_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Stream_Cursor_Input>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Auctions_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Collection_OffersArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_By_PkArgs = {
  collection_id: Scalars['String'];
  collection_offer_id: Scalars['String'];
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Stream_Cursor_Input>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Collection_Offers_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_ListingsArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Listings_By_PkArgs = {
  listing_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Listings_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Stream_Cursor_Input>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Listings_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Token_OffersArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_By_PkArgs = {
  offer_id: Scalars['String'];
  token_data_id: Scalars['String'];
};


export type Subscription_RootNft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Stream_Cursor_Input>>;
  where?: InputMaybe<Nft_Marketplace_V2_Current_Nft_Marketplace_Token_Offers_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Nft_Marketplace_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Nft_Marketplace_V2_Nft_Marketplace_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Marketplace_V2_Nft_Marketplace_Activities_Order_By>>;
  where?: InputMaybe<Nft_Marketplace_V2_Nft_Marketplace_Activities_Bool_Exp>;
};


export type Subscription_RootNft_Marketplace_V2_Nft_Marketplace_Activities_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootNft_Marketplace_V2_Nft_Marketplace_Activities_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Nft_Marketplace_V2_Nft_Marketplace_Activities_Stream_Cursor_Input>>;
  where?: InputMaybe<Nft_Marketplace_V2_Nft_Marketplace_Activities_Bool_Exp>;
};


export type Subscription_RootNft_Metadata_Crawler_Parsed_Asset_UrisArgs = {
  distinct_on?: InputMaybe<Array<Nft_Metadata_Crawler_Parsed_Asset_Uris_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Nft_Metadata_Crawler_Parsed_Asset_Uris_Order_By>>;
  where?: InputMaybe<Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp>;
};


export type Subscription_RootNft_Metadata_Crawler_Parsed_Asset_Uris_By_PkArgs = {
  asset_uri: Scalars['String'];
};


export type Subscription_RootNft_Metadata_Crawler_Parsed_Asset_Uris_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Nft_Metadata_Crawler_Parsed_Asset_Uris_Stream_Cursor_Input>>;
  where?: InputMaybe<Nft_Metadata_Crawler_Parsed_Asset_Uris_Bool_Exp>;
};


export type Subscription_RootNum_Active_Delegator_Per_PoolArgs = {
  distinct_on?: InputMaybe<Array<Num_Active_Delegator_Per_Pool_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Num_Active_Delegator_Per_Pool_Order_By>>;
  where?: InputMaybe<Num_Active_Delegator_Per_Pool_Bool_Exp>;
};


export type Subscription_RootNum_Active_Delegator_Per_Pool_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Num_Active_Delegator_Per_Pool_Stream_Cursor_Input>>;
  where?: InputMaybe<Num_Active_Delegator_Per_Pool_Bool_Exp>;
};


export type Subscription_RootProcessor_StatusArgs = {
  distinct_on?: InputMaybe<Array<Processor_Status_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Processor_Status_Order_By>>;
  where?: InputMaybe<Processor_Status_Bool_Exp>;
};


export type Subscription_RootProcessor_Status_By_PkArgs = {
  processor: Scalars['String'];
};


export type Subscription_RootProcessor_Status_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Processor_Status_Stream_Cursor_Input>>;
  where?: InputMaybe<Processor_Status_Bool_Exp>;
};


export type Subscription_RootProposal_VotesArgs = {
  distinct_on?: InputMaybe<Array<Proposal_Votes_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Proposal_Votes_Order_By>>;
  where?: InputMaybe<Proposal_Votes_Bool_Exp>;
};


export type Subscription_RootProposal_Votes_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Proposal_Votes_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Proposal_Votes_Order_By>>;
  where?: InputMaybe<Proposal_Votes_Bool_Exp>;
};


export type Subscription_RootProposal_Votes_By_PkArgs = {
  proposal_id: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
  voter_address: Scalars['String'];
};


export type Subscription_RootProposal_Votes_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Proposal_Votes_Stream_Cursor_Input>>;
  where?: InputMaybe<Proposal_Votes_Bool_Exp>;
};


export type Subscription_RootTable_ItemsArgs = {
  distinct_on?: InputMaybe<Array<Table_Items_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Table_Items_Order_By>>;
  where?: InputMaybe<Table_Items_Bool_Exp>;
};


export type Subscription_RootTable_Items_By_PkArgs = {
  transaction_version: Scalars['bigint'];
  write_set_change_index: Scalars['bigint'];
};


export type Subscription_RootTable_Items_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Table_Items_Stream_Cursor_Input>>;
  where?: InputMaybe<Table_Items_Bool_Exp>;
};


export type Subscription_RootTable_MetadatasArgs = {
  distinct_on?: InputMaybe<Array<Table_Metadatas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Table_Metadatas_Order_By>>;
  where?: InputMaybe<Table_Metadatas_Bool_Exp>;
};


export type Subscription_RootTable_Metadatas_By_PkArgs = {
  handle: Scalars['String'];
};


export type Subscription_RootTable_Metadatas_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Table_Metadatas_Stream_Cursor_Input>>;
  where?: InputMaybe<Table_Metadatas_Bool_Exp>;
};


export type Subscription_RootToken_ActivitiesArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


export type Subscription_RootToken_Activities_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_Order_By>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


export type Subscription_RootToken_Activities_By_PkArgs = {
  event_account_address: Scalars['String'];
  event_creation_number: Scalars['bigint'];
  event_sequence_number: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootToken_Activities_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Token_Activities_Stream_Cursor_Input>>;
  where?: InputMaybe<Token_Activities_Bool_Exp>;
};


export type Subscription_RootToken_Activities_V2Args = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


export type Subscription_RootToken_Activities_V2_AggregateArgs = {
  distinct_on?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Activities_V2_Order_By>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


export type Subscription_RootToken_Activities_V2_By_PkArgs = {
  event_index: Scalars['bigint'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootToken_Activities_V2_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Token_Activities_V2_Stream_Cursor_Input>>;
  where?: InputMaybe<Token_Activities_V2_Bool_Exp>;
};


export type Subscription_RootToken_DatasArgs = {
  distinct_on?: InputMaybe<Array<Token_Datas_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Datas_Order_By>>;
  where?: InputMaybe<Token_Datas_Bool_Exp>;
};


export type Subscription_RootToken_Datas_By_PkArgs = {
  token_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootToken_Datas_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Token_Datas_Stream_Cursor_Input>>;
  where?: InputMaybe<Token_Datas_Bool_Exp>;
};


export type Subscription_RootToken_OwnershipsArgs = {
  distinct_on?: InputMaybe<Array<Token_Ownerships_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Token_Ownerships_Order_By>>;
  where?: InputMaybe<Token_Ownerships_Bool_Exp>;
};


export type Subscription_RootToken_Ownerships_By_PkArgs = {
  property_version: Scalars['numeric'];
  table_handle: Scalars['String'];
  token_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootToken_Ownerships_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Token_Ownerships_Stream_Cursor_Input>>;
  where?: InputMaybe<Token_Ownerships_Bool_Exp>;
};


export type Subscription_RootTokensArgs = {
  distinct_on?: InputMaybe<Array<Tokens_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Tokens_Order_By>>;
  where?: InputMaybe<Tokens_Bool_Exp>;
};


export type Subscription_RootTokens_By_PkArgs = {
  property_version: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
  transaction_version: Scalars['bigint'];
};


export type Subscription_RootTokens_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<Tokens_Stream_Cursor_Input>>;
  where?: InputMaybe<Tokens_Bool_Exp>;
};


export type Subscription_RootUser_TransactionsArgs = {
  distinct_on?: InputMaybe<Array<User_Transactions_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<User_Transactions_Order_By>>;
  where?: InputMaybe<User_Transactions_Bool_Exp>;
};


export type Subscription_RootUser_Transactions_By_PkArgs = {
  version: Scalars['bigint'];
};


export type Subscription_RootUser_Transactions_StreamArgs = {
  batch_size: Scalars['Int'];
  cursor: Array<InputMaybe<User_Transactions_Stream_Cursor_Input>>;
  where?: InputMaybe<User_Transactions_Bool_Exp>;
};

/** columns and relationships of "table_items" */
export type Table_Items = {
  __typename?: 'table_items';
  decoded_key: Scalars['jsonb'];
  decoded_value?: Maybe<Scalars['jsonb']>;
  key: Scalars['String'];
  table_handle: Scalars['String'];
  transaction_version: Scalars['bigint'];
  write_set_change_index: Scalars['bigint'];
};


/** columns and relationships of "table_items" */
export type Table_ItemsDecoded_KeyArgs = {
  path?: InputMaybe<Scalars['String']>;
};


/** columns and relationships of "table_items" */
export type Table_ItemsDecoded_ValueArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "table_items". All fields are combined with a logical 'AND'. */
export type Table_Items_Bool_Exp = {
  _and?: InputMaybe<Array<Table_Items_Bool_Exp>>;
  _not?: InputMaybe<Table_Items_Bool_Exp>;
  _or?: InputMaybe<Array<Table_Items_Bool_Exp>>;
  decoded_key?: InputMaybe<Jsonb_Comparison_Exp>;
  decoded_value?: InputMaybe<Jsonb_Comparison_Exp>;
  key?: InputMaybe<String_Comparison_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  write_set_change_index?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "table_items". */
export type Table_Items_Order_By = {
  decoded_key?: InputMaybe<Order_By>;
  decoded_value?: InputMaybe<Order_By>;
  key?: InputMaybe<Order_By>;
  table_handle?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  write_set_change_index?: InputMaybe<Order_By>;
};

/** select columns of table "table_items" */
export enum Table_Items_Select_Column {
  /** column name */
  DecodedKey = 'decoded_key',
  /** column name */
  DecodedValue = 'decoded_value',
  /** column name */
  Key = 'key',
  /** column name */
  TableHandle = 'table_handle',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  WriteSetChangeIndex = 'write_set_change_index'
}

/** Streaming cursor of the table "table_items" */
export type Table_Items_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Table_Items_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Table_Items_Stream_Cursor_Value_Input = {
  decoded_key?: InputMaybe<Scalars['jsonb']>;
  decoded_value?: InputMaybe<Scalars['jsonb']>;
  key?: InputMaybe<Scalars['String']>;
  table_handle?: InputMaybe<Scalars['String']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  write_set_change_index?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "table_metadatas" */
export type Table_Metadatas = {
  __typename?: 'table_metadatas';
  handle: Scalars['String'];
  key_type: Scalars['String'];
  value_type: Scalars['String'];
};

/** Boolean expression to filter rows from the table "table_metadatas". All fields are combined with a logical 'AND'. */
export type Table_Metadatas_Bool_Exp = {
  _and?: InputMaybe<Array<Table_Metadatas_Bool_Exp>>;
  _not?: InputMaybe<Table_Metadatas_Bool_Exp>;
  _or?: InputMaybe<Array<Table_Metadatas_Bool_Exp>>;
  handle?: InputMaybe<String_Comparison_Exp>;
  key_type?: InputMaybe<String_Comparison_Exp>;
  value_type?: InputMaybe<String_Comparison_Exp>;
};

/** Ordering options when selecting data from "table_metadatas". */
export type Table_Metadatas_Order_By = {
  handle?: InputMaybe<Order_By>;
  key_type?: InputMaybe<Order_By>;
  value_type?: InputMaybe<Order_By>;
};

/** select columns of table "table_metadatas" */
export enum Table_Metadatas_Select_Column {
  /** column name */
  Handle = 'handle',
  /** column name */
  KeyType = 'key_type',
  /** column name */
  ValueType = 'value_type'
}

/** Streaming cursor of the table "table_metadatas" */
export type Table_Metadatas_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Table_Metadatas_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Table_Metadatas_Stream_Cursor_Value_Input = {
  handle?: InputMaybe<Scalars['String']>;
  key_type?: InputMaybe<Scalars['String']>;
  value_type?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to compare columns of type "timestamp". All fields are combined with logical 'AND'. */
export type Timestamp_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['timestamp']>;
  _gt?: InputMaybe<Scalars['timestamp']>;
  _gte?: InputMaybe<Scalars['timestamp']>;
  _in?: InputMaybe<Array<Scalars['timestamp']>>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  _lt?: InputMaybe<Scalars['timestamp']>;
  _lte?: InputMaybe<Scalars['timestamp']>;
  _neq?: InputMaybe<Scalars['timestamp']>;
  _nin?: InputMaybe<Array<Scalars['timestamp']>>;
};

/** Boolean expression to compare columns of type "timestamptz". All fields are combined with logical 'AND'. */
export type Timestamptz_Comparison_Exp = {
  _eq?: InputMaybe<Scalars['timestamptz']>;
  _gt?: InputMaybe<Scalars['timestamptz']>;
  _gte?: InputMaybe<Scalars['timestamptz']>;
  _in?: InputMaybe<Array<Scalars['timestamptz']>>;
  _is_null?: InputMaybe<Scalars['Boolean']>;
  _lt?: InputMaybe<Scalars['timestamptz']>;
  _lte?: InputMaybe<Scalars['timestamptz']>;
  _neq?: InputMaybe<Scalars['timestamptz']>;
  _nin?: InputMaybe<Array<Scalars['timestamptz']>>;
};

/** columns and relationships of "token_activities" */
export type Token_Activities = {
  __typename?: 'token_activities';
  /** An array relationship */
  aptos_names_owner: Array<Current_Aptos_Names>;
  /** An array relationship */
  aptos_names_to: Array<Current_Aptos_Names>;
  coin_amount?: Maybe<Scalars['numeric']>;
  coin_type?: Maybe<Scalars['String']>;
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas>;
  event_account_address: Scalars['String'];
  event_creation_number: Scalars['bigint'];
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number: Scalars['bigint'];
  from_address?: Maybe<Scalars['String']>;
  name: Scalars['String'];
  property_version: Scalars['numeric'];
  to_address?: Maybe<Scalars['String']>;
  token_amount: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
  transfer_type: Scalars['String'];
};


/** columns and relationships of "token_activities" */
export type Token_ActivitiesAptos_Names_OwnerArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};


/** columns and relationships of "token_activities" */
export type Token_ActivitiesAptos_Names_ToArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};

/** aggregated selection of "token_activities" */
export type Token_Activities_Aggregate = {
  __typename?: 'token_activities_aggregate';
  aggregate?: Maybe<Token_Activities_Aggregate_Fields>;
  nodes: Array<Token_Activities>;
};

/** aggregate fields of "token_activities" */
export type Token_Activities_Aggregate_Fields = {
  __typename?: 'token_activities_aggregate_fields';
  avg?: Maybe<Token_Activities_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Token_Activities_Max_Fields>;
  min?: Maybe<Token_Activities_Min_Fields>;
  stddev?: Maybe<Token_Activities_Stddev_Fields>;
  stddev_pop?: Maybe<Token_Activities_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Token_Activities_Stddev_Samp_Fields>;
  sum?: Maybe<Token_Activities_Sum_Fields>;
  var_pop?: Maybe<Token_Activities_Var_Pop_Fields>;
  var_samp?: Maybe<Token_Activities_Var_Samp_Fields>;
  variance?: Maybe<Token_Activities_Variance_Fields>;
};


/** aggregate fields of "token_activities" */
export type Token_Activities_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Token_Activities_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** order by aggregate values of table "token_activities" */
export type Token_Activities_Aggregate_Order_By = {
  avg?: InputMaybe<Token_Activities_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Token_Activities_Max_Order_By>;
  min?: InputMaybe<Token_Activities_Min_Order_By>;
  stddev?: InputMaybe<Token_Activities_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Token_Activities_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Token_Activities_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Token_Activities_Sum_Order_By>;
  var_pop?: InputMaybe<Token_Activities_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Token_Activities_Var_Samp_Order_By>;
  variance?: InputMaybe<Token_Activities_Variance_Order_By>;
};

/** aggregate avg on columns */
export type Token_Activities_Avg_Fields = {
  __typename?: 'token_activities_avg_fields';
  coin_amount?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by avg() on columns of table "token_activities" */
export type Token_Activities_Avg_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "token_activities". All fields are combined with a logical 'AND'. */
export type Token_Activities_Bool_Exp = {
  _and?: InputMaybe<Array<Token_Activities_Bool_Exp>>;
  _not?: InputMaybe<Token_Activities_Bool_Exp>;
  _or?: InputMaybe<Array<Token_Activities_Bool_Exp>>;
  aptos_names_owner?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  aptos_names_to?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  coin_amount?: InputMaybe<Numeric_Comparison_Exp>;
  coin_type?: InputMaybe<String_Comparison_Exp>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_Bool_Exp>;
  event_account_address?: InputMaybe<String_Comparison_Exp>;
  event_creation_number?: InputMaybe<Bigint_Comparison_Exp>;
  event_index?: InputMaybe<Bigint_Comparison_Exp>;
  event_sequence_number?: InputMaybe<Bigint_Comparison_Exp>;
  from_address?: InputMaybe<String_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  property_version?: InputMaybe<Numeric_Comparison_Exp>;
  to_address?: InputMaybe<String_Comparison_Exp>;
  token_amount?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  transfer_type?: InputMaybe<String_Comparison_Exp>;
};

/** aggregate max on columns */
export type Token_Activities_Max_Fields = {
  __typename?: 'token_activities_max_fields';
  coin_amount?: Maybe<Scalars['numeric']>;
  coin_type?: Maybe<Scalars['String']>;
  collection_data_id_hash?: Maybe<Scalars['String']>;
  collection_name?: Maybe<Scalars['String']>;
  creator_address?: Maybe<Scalars['String']>;
  event_account_address?: Maybe<Scalars['String']>;
  event_creation_number?: Maybe<Scalars['bigint']>;
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number?: Maybe<Scalars['bigint']>;
  from_address?: Maybe<Scalars['String']>;
  name?: Maybe<Scalars['String']>;
  property_version?: Maybe<Scalars['numeric']>;
  to_address?: Maybe<Scalars['String']>;
  token_amount?: Maybe<Scalars['numeric']>;
  token_data_id_hash?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
  transfer_type?: Maybe<Scalars['String']>;
};

/** order by max() on columns of table "token_activities" */
export type Token_Activities_Max_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  from_address?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  to_address?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  transfer_type?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Token_Activities_Min_Fields = {
  __typename?: 'token_activities_min_fields';
  coin_amount?: Maybe<Scalars['numeric']>;
  coin_type?: Maybe<Scalars['String']>;
  collection_data_id_hash?: Maybe<Scalars['String']>;
  collection_name?: Maybe<Scalars['String']>;
  creator_address?: Maybe<Scalars['String']>;
  event_account_address?: Maybe<Scalars['String']>;
  event_creation_number?: Maybe<Scalars['bigint']>;
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number?: Maybe<Scalars['bigint']>;
  from_address?: Maybe<Scalars['String']>;
  name?: Maybe<Scalars['String']>;
  property_version?: Maybe<Scalars['numeric']>;
  to_address?: Maybe<Scalars['String']>;
  token_amount?: Maybe<Scalars['numeric']>;
  token_data_id_hash?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
  transfer_type?: Maybe<Scalars['String']>;
};

/** order by min() on columns of table "token_activities" */
export type Token_Activities_Min_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  from_address?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  to_address?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  transfer_type?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "token_activities". */
export type Token_Activities_Order_By = {
  aptos_names_owner_aggregate?: InputMaybe<Current_Aptos_Names_Aggregate_Order_By>;
  aptos_names_to_aggregate?: InputMaybe<Current_Aptos_Names_Aggregate_Order_By>;
  coin_amount?: InputMaybe<Order_By>;
  coin_type?: InputMaybe<Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  from_address?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  to_address?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  transfer_type?: InputMaybe<Order_By>;
};

/** select columns of table "token_activities" */
export enum Token_Activities_Select_Column {
  /** column name */
  CoinAmount = 'coin_amount',
  /** column name */
  CoinType = 'coin_type',
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  EventAccountAddress = 'event_account_address',
  /** column name */
  EventCreationNumber = 'event_creation_number',
  /** column name */
  EventIndex = 'event_index',
  /** column name */
  EventSequenceNumber = 'event_sequence_number',
  /** column name */
  FromAddress = 'from_address',
  /** column name */
  Name = 'name',
  /** column name */
  PropertyVersion = 'property_version',
  /** column name */
  ToAddress = 'to_address',
  /** column name */
  TokenAmount = 'token_amount',
  /** column name */
  TokenDataIdHash = 'token_data_id_hash',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  TransferType = 'transfer_type'
}

/** aggregate stddev on columns */
export type Token_Activities_Stddev_Fields = {
  __typename?: 'token_activities_stddev_fields';
  coin_amount?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev() on columns of table "token_activities" */
export type Token_Activities_Stddev_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_pop on columns */
export type Token_Activities_Stddev_Pop_Fields = {
  __typename?: 'token_activities_stddev_pop_fields';
  coin_amount?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_pop() on columns of table "token_activities" */
export type Token_Activities_Stddev_Pop_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_samp on columns */
export type Token_Activities_Stddev_Samp_Fields = {
  __typename?: 'token_activities_stddev_samp_fields';
  coin_amount?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_samp() on columns of table "token_activities" */
export type Token_Activities_Stddev_Samp_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "token_activities" */
export type Token_Activities_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Token_Activities_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Token_Activities_Stream_Cursor_Value_Input = {
  coin_amount?: InputMaybe<Scalars['numeric']>;
  coin_type?: InputMaybe<Scalars['String']>;
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  event_account_address?: InputMaybe<Scalars['String']>;
  event_creation_number?: InputMaybe<Scalars['bigint']>;
  event_index?: InputMaybe<Scalars['bigint']>;
  event_sequence_number?: InputMaybe<Scalars['bigint']>;
  from_address?: InputMaybe<Scalars['String']>;
  name?: InputMaybe<Scalars['String']>;
  property_version?: InputMaybe<Scalars['numeric']>;
  to_address?: InputMaybe<Scalars['String']>;
  token_amount?: InputMaybe<Scalars['numeric']>;
  token_data_id_hash?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  transfer_type?: InputMaybe<Scalars['String']>;
};

/** aggregate sum on columns */
export type Token_Activities_Sum_Fields = {
  __typename?: 'token_activities_sum_fields';
  coin_amount?: Maybe<Scalars['numeric']>;
  event_creation_number?: Maybe<Scalars['bigint']>;
  event_index?: Maybe<Scalars['bigint']>;
  event_sequence_number?: Maybe<Scalars['bigint']>;
  property_version?: Maybe<Scalars['numeric']>;
  token_amount?: Maybe<Scalars['numeric']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** order by sum() on columns of table "token_activities" */
export type Token_Activities_Sum_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "token_activities_v2" */
export type Token_Activities_V2 = {
  __typename?: 'token_activities_v2';
  after_value?: Maybe<Scalars['String']>;
  /** An array relationship */
  aptos_names_from: Array<Current_Aptos_Names>;
  /** An array relationship */
  aptos_names_to: Array<Current_Aptos_Names>;
  before_value?: Maybe<Scalars['String']>;
  /** An object relationship */
  current_token_data?: Maybe<Current_Token_Datas_V2>;
  entry_function_id_str?: Maybe<Scalars['String']>;
  event_account_address: Scalars['String'];
  event_index: Scalars['bigint'];
  from_address?: Maybe<Scalars['String']>;
  is_fungible_v2?: Maybe<Scalars['Boolean']>;
  property_version_v1: Scalars['numeric'];
  to_address?: Maybe<Scalars['String']>;
  token_amount: Scalars['numeric'];
  token_data_id: Scalars['String'];
  token_standard: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
  type: Scalars['String'];
};


/** columns and relationships of "token_activities_v2" */
export type Token_Activities_V2Aptos_Names_FromArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};


/** columns and relationships of "token_activities_v2" */
export type Token_Activities_V2Aptos_Names_ToArgs = {
  distinct_on?: InputMaybe<Array<Current_Aptos_Names_Select_Column>>;
  limit?: InputMaybe<Scalars['Int']>;
  offset?: InputMaybe<Scalars['Int']>;
  order_by?: InputMaybe<Array<Current_Aptos_Names_Order_By>>;
  where?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
};

/** aggregated selection of "token_activities_v2" */
export type Token_Activities_V2_Aggregate = {
  __typename?: 'token_activities_v2_aggregate';
  aggregate?: Maybe<Token_Activities_V2_Aggregate_Fields>;
  nodes: Array<Token_Activities_V2>;
};

/** aggregate fields of "token_activities_v2" */
export type Token_Activities_V2_Aggregate_Fields = {
  __typename?: 'token_activities_v2_aggregate_fields';
  avg?: Maybe<Token_Activities_V2_Avg_Fields>;
  count: Scalars['Int'];
  max?: Maybe<Token_Activities_V2_Max_Fields>;
  min?: Maybe<Token_Activities_V2_Min_Fields>;
  stddev?: Maybe<Token_Activities_V2_Stddev_Fields>;
  stddev_pop?: Maybe<Token_Activities_V2_Stddev_Pop_Fields>;
  stddev_samp?: Maybe<Token_Activities_V2_Stddev_Samp_Fields>;
  sum?: Maybe<Token_Activities_V2_Sum_Fields>;
  var_pop?: Maybe<Token_Activities_V2_Var_Pop_Fields>;
  var_samp?: Maybe<Token_Activities_V2_Var_Samp_Fields>;
  variance?: Maybe<Token_Activities_V2_Variance_Fields>;
};


/** aggregate fields of "token_activities_v2" */
export type Token_Activities_V2_Aggregate_FieldsCountArgs = {
  columns?: InputMaybe<Array<Token_Activities_V2_Select_Column>>;
  distinct?: InputMaybe<Scalars['Boolean']>;
};

/** order by aggregate values of table "token_activities_v2" */
export type Token_Activities_V2_Aggregate_Order_By = {
  avg?: InputMaybe<Token_Activities_V2_Avg_Order_By>;
  count?: InputMaybe<Order_By>;
  max?: InputMaybe<Token_Activities_V2_Max_Order_By>;
  min?: InputMaybe<Token_Activities_V2_Min_Order_By>;
  stddev?: InputMaybe<Token_Activities_V2_Stddev_Order_By>;
  stddev_pop?: InputMaybe<Token_Activities_V2_Stddev_Pop_Order_By>;
  stddev_samp?: InputMaybe<Token_Activities_V2_Stddev_Samp_Order_By>;
  sum?: InputMaybe<Token_Activities_V2_Sum_Order_By>;
  var_pop?: InputMaybe<Token_Activities_V2_Var_Pop_Order_By>;
  var_samp?: InputMaybe<Token_Activities_V2_Var_Samp_Order_By>;
  variance?: InputMaybe<Token_Activities_V2_Variance_Order_By>;
};

/** aggregate avg on columns */
export type Token_Activities_V2_Avg_Fields = {
  __typename?: 'token_activities_v2_avg_fields';
  event_index?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by avg() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Avg_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Boolean expression to filter rows from the table "token_activities_v2". All fields are combined with a logical 'AND'. */
export type Token_Activities_V2_Bool_Exp = {
  _and?: InputMaybe<Array<Token_Activities_V2_Bool_Exp>>;
  _not?: InputMaybe<Token_Activities_V2_Bool_Exp>;
  _or?: InputMaybe<Array<Token_Activities_V2_Bool_Exp>>;
  after_value?: InputMaybe<String_Comparison_Exp>;
  aptos_names_from?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  aptos_names_to?: InputMaybe<Current_Aptos_Names_Bool_Exp>;
  before_value?: InputMaybe<String_Comparison_Exp>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Bool_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  event_account_address?: InputMaybe<String_Comparison_Exp>;
  event_index?: InputMaybe<Bigint_Comparison_Exp>;
  from_address?: InputMaybe<String_Comparison_Exp>;
  is_fungible_v2?: InputMaybe<Boolean_Comparison_Exp>;
  property_version_v1?: InputMaybe<Numeric_Comparison_Exp>;
  to_address?: InputMaybe<String_Comparison_Exp>;
  token_amount?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id?: InputMaybe<String_Comparison_Exp>;
  token_standard?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  type?: InputMaybe<String_Comparison_Exp>;
};

/** aggregate max on columns */
export type Token_Activities_V2_Max_Fields = {
  __typename?: 'token_activities_v2_max_fields';
  after_value?: Maybe<Scalars['String']>;
  before_value?: Maybe<Scalars['String']>;
  entry_function_id_str?: Maybe<Scalars['String']>;
  event_account_address?: Maybe<Scalars['String']>;
  event_index?: Maybe<Scalars['bigint']>;
  from_address?: Maybe<Scalars['String']>;
  property_version_v1?: Maybe<Scalars['numeric']>;
  to_address?: Maybe<Scalars['String']>;
  token_amount?: Maybe<Scalars['numeric']>;
  token_data_id?: Maybe<Scalars['String']>;
  token_standard?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
  type?: Maybe<Scalars['String']>;
};

/** order by max() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Max_Order_By = {
  after_value?: InputMaybe<Order_By>;
  before_value?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  from_address?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  to_address?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** aggregate min on columns */
export type Token_Activities_V2_Min_Fields = {
  __typename?: 'token_activities_v2_min_fields';
  after_value?: Maybe<Scalars['String']>;
  before_value?: Maybe<Scalars['String']>;
  entry_function_id_str?: Maybe<Scalars['String']>;
  event_account_address?: Maybe<Scalars['String']>;
  event_index?: Maybe<Scalars['bigint']>;
  from_address?: Maybe<Scalars['String']>;
  property_version_v1?: Maybe<Scalars['numeric']>;
  to_address?: Maybe<Scalars['String']>;
  token_amount?: Maybe<Scalars['numeric']>;
  token_data_id?: Maybe<Scalars['String']>;
  token_standard?: Maybe<Scalars['String']>;
  transaction_timestamp?: Maybe<Scalars['timestamp']>;
  transaction_version?: Maybe<Scalars['bigint']>;
  type?: Maybe<Scalars['String']>;
};

/** order by min() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Min_Order_By = {
  after_value?: InputMaybe<Order_By>;
  before_value?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  from_address?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  to_address?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** Ordering options when selecting data from "token_activities_v2". */
export type Token_Activities_V2_Order_By = {
  after_value?: InputMaybe<Order_By>;
  aptos_names_from_aggregate?: InputMaybe<Current_Aptos_Names_Aggregate_Order_By>;
  aptos_names_to_aggregate?: InputMaybe<Current_Aptos_Names_Aggregate_Order_By>;
  before_value?: InputMaybe<Order_By>;
  current_token_data?: InputMaybe<Current_Token_Datas_V2_Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  event_account_address?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  from_address?: InputMaybe<Order_By>;
  is_fungible_v2?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  to_address?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  token_data_id?: InputMaybe<Order_By>;
  token_standard?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  type?: InputMaybe<Order_By>;
};

/** select columns of table "token_activities_v2" */
export enum Token_Activities_V2_Select_Column {
  /** column name */
  AfterValue = 'after_value',
  /** column name */
  BeforeValue = 'before_value',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  EventAccountAddress = 'event_account_address',
  /** column name */
  EventIndex = 'event_index',
  /** column name */
  FromAddress = 'from_address',
  /** column name */
  IsFungibleV2 = 'is_fungible_v2',
  /** column name */
  PropertyVersionV1 = 'property_version_v1',
  /** column name */
  ToAddress = 'to_address',
  /** column name */
  TokenAmount = 'token_amount',
  /** column name */
  TokenDataId = 'token_data_id',
  /** column name */
  TokenStandard = 'token_standard',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  Type = 'type'
}

/** aggregate stddev on columns */
export type Token_Activities_V2_Stddev_Fields = {
  __typename?: 'token_activities_v2_stddev_fields';
  event_index?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Stddev_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_pop on columns */
export type Token_Activities_V2_Stddev_Pop_Fields = {
  __typename?: 'token_activities_v2_stddev_pop_fields';
  event_index?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_pop() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Stddev_Pop_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate stddev_samp on columns */
export type Token_Activities_V2_Stddev_Samp_Fields = {
  __typename?: 'token_activities_v2_stddev_samp_fields';
  event_index?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by stddev_samp() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Stddev_Samp_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** Streaming cursor of the table "token_activities_v2" */
export type Token_Activities_V2_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Token_Activities_V2_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Token_Activities_V2_Stream_Cursor_Value_Input = {
  after_value?: InputMaybe<Scalars['String']>;
  before_value?: InputMaybe<Scalars['String']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  event_account_address?: InputMaybe<Scalars['String']>;
  event_index?: InputMaybe<Scalars['bigint']>;
  from_address?: InputMaybe<Scalars['String']>;
  is_fungible_v2?: InputMaybe<Scalars['Boolean']>;
  property_version_v1?: InputMaybe<Scalars['numeric']>;
  to_address?: InputMaybe<Scalars['String']>;
  token_amount?: InputMaybe<Scalars['numeric']>;
  token_data_id?: InputMaybe<Scalars['String']>;
  token_standard?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  type?: InputMaybe<Scalars['String']>;
};

/** aggregate sum on columns */
export type Token_Activities_V2_Sum_Fields = {
  __typename?: 'token_activities_v2_sum_fields';
  event_index?: Maybe<Scalars['bigint']>;
  property_version_v1?: Maybe<Scalars['numeric']>;
  token_amount?: Maybe<Scalars['numeric']>;
  transaction_version?: Maybe<Scalars['bigint']>;
};

/** order by sum() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Sum_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate var_pop on columns */
export type Token_Activities_V2_Var_Pop_Fields = {
  __typename?: 'token_activities_v2_var_pop_fields';
  event_index?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by var_pop() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Var_Pop_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate var_samp on columns */
export type Token_Activities_V2_Var_Samp_Fields = {
  __typename?: 'token_activities_v2_var_samp_fields';
  event_index?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by var_samp() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Var_Samp_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate variance on columns */
export type Token_Activities_V2_Variance_Fields = {
  __typename?: 'token_activities_v2_variance_fields';
  event_index?: Maybe<Scalars['Float']>;
  property_version_v1?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by variance() on columns of table "token_activities_v2" */
export type Token_Activities_V2_Variance_Order_By = {
  event_index?: InputMaybe<Order_By>;
  property_version_v1?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate var_pop on columns */
export type Token_Activities_Var_Pop_Fields = {
  __typename?: 'token_activities_var_pop_fields';
  coin_amount?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by var_pop() on columns of table "token_activities" */
export type Token_Activities_Var_Pop_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate var_samp on columns */
export type Token_Activities_Var_Samp_Fields = {
  __typename?: 'token_activities_var_samp_fields';
  coin_amount?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by var_samp() on columns of table "token_activities" */
export type Token_Activities_Var_Samp_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** aggregate variance on columns */
export type Token_Activities_Variance_Fields = {
  __typename?: 'token_activities_variance_fields';
  coin_amount?: Maybe<Scalars['Float']>;
  event_creation_number?: Maybe<Scalars['Float']>;
  event_index?: Maybe<Scalars['Float']>;
  event_sequence_number?: Maybe<Scalars['Float']>;
  property_version?: Maybe<Scalars['Float']>;
  token_amount?: Maybe<Scalars['Float']>;
  transaction_version?: Maybe<Scalars['Float']>;
};

/** order by variance() on columns of table "token_activities" */
export type Token_Activities_Variance_Order_By = {
  coin_amount?: InputMaybe<Order_By>;
  event_creation_number?: InputMaybe<Order_By>;
  event_index?: InputMaybe<Order_By>;
  event_sequence_number?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_amount?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** columns and relationships of "token_datas" */
export type Token_Datas = {
  __typename?: 'token_datas';
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  default_properties: Scalars['jsonb'];
  description: Scalars['String'];
  description_mutable: Scalars['Boolean'];
  largest_property_version: Scalars['numeric'];
  maximum: Scalars['numeric'];
  maximum_mutable: Scalars['Boolean'];
  metadata_uri: Scalars['String'];
  name: Scalars['String'];
  payee_address: Scalars['String'];
  properties_mutable: Scalars['Boolean'];
  royalty_mutable: Scalars['Boolean'];
  royalty_points_denominator: Scalars['numeric'];
  royalty_points_numerator: Scalars['numeric'];
  supply: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
  uri_mutable: Scalars['Boolean'];
};


/** columns and relationships of "token_datas" */
export type Token_DatasDefault_PropertiesArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "token_datas". All fields are combined with a logical 'AND'. */
export type Token_Datas_Bool_Exp = {
  _and?: InputMaybe<Array<Token_Datas_Bool_Exp>>;
  _not?: InputMaybe<Token_Datas_Bool_Exp>;
  _or?: InputMaybe<Array<Token_Datas_Bool_Exp>>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  default_properties?: InputMaybe<Jsonb_Comparison_Exp>;
  description?: InputMaybe<String_Comparison_Exp>;
  description_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  largest_property_version?: InputMaybe<Numeric_Comparison_Exp>;
  maximum?: InputMaybe<Numeric_Comparison_Exp>;
  maximum_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  metadata_uri?: InputMaybe<String_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  payee_address?: InputMaybe<String_Comparison_Exp>;
  properties_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  royalty_mutable?: InputMaybe<Boolean_Comparison_Exp>;
  royalty_points_denominator?: InputMaybe<Numeric_Comparison_Exp>;
  royalty_points_numerator?: InputMaybe<Numeric_Comparison_Exp>;
  supply?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
  uri_mutable?: InputMaybe<Boolean_Comparison_Exp>;
};

/** Ordering options when selecting data from "token_datas". */
export type Token_Datas_Order_By = {
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  default_properties?: InputMaybe<Order_By>;
  description?: InputMaybe<Order_By>;
  description_mutable?: InputMaybe<Order_By>;
  largest_property_version?: InputMaybe<Order_By>;
  maximum?: InputMaybe<Order_By>;
  maximum_mutable?: InputMaybe<Order_By>;
  metadata_uri?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  payee_address?: InputMaybe<Order_By>;
  properties_mutable?: InputMaybe<Order_By>;
  royalty_mutable?: InputMaybe<Order_By>;
  royalty_points_denominator?: InputMaybe<Order_By>;
  royalty_points_numerator?: InputMaybe<Order_By>;
  supply?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
  uri_mutable?: InputMaybe<Order_By>;
};

/** select columns of table "token_datas" */
export enum Token_Datas_Select_Column {
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  DefaultProperties = 'default_properties',
  /** column name */
  Description = 'description',
  /** column name */
  DescriptionMutable = 'description_mutable',
  /** column name */
  LargestPropertyVersion = 'largest_property_version',
  /** column name */
  Maximum = 'maximum',
  /** column name */
  MaximumMutable = 'maximum_mutable',
  /** column name */
  MetadataUri = 'metadata_uri',
  /** column name */
  Name = 'name',
  /** column name */
  PayeeAddress = 'payee_address',
  /** column name */
  PropertiesMutable = 'properties_mutable',
  /** column name */
  RoyaltyMutable = 'royalty_mutable',
  /** column name */
  RoyaltyPointsDenominator = 'royalty_points_denominator',
  /** column name */
  RoyaltyPointsNumerator = 'royalty_points_numerator',
  /** column name */
  Supply = 'supply',
  /** column name */
  TokenDataIdHash = 'token_data_id_hash',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version',
  /** column name */
  UriMutable = 'uri_mutable'
}

/** Streaming cursor of the table "token_datas" */
export type Token_Datas_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Token_Datas_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Token_Datas_Stream_Cursor_Value_Input = {
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  default_properties?: InputMaybe<Scalars['jsonb']>;
  description?: InputMaybe<Scalars['String']>;
  description_mutable?: InputMaybe<Scalars['Boolean']>;
  largest_property_version?: InputMaybe<Scalars['numeric']>;
  maximum?: InputMaybe<Scalars['numeric']>;
  maximum_mutable?: InputMaybe<Scalars['Boolean']>;
  metadata_uri?: InputMaybe<Scalars['String']>;
  name?: InputMaybe<Scalars['String']>;
  payee_address?: InputMaybe<Scalars['String']>;
  properties_mutable?: InputMaybe<Scalars['Boolean']>;
  royalty_mutable?: InputMaybe<Scalars['Boolean']>;
  royalty_points_denominator?: InputMaybe<Scalars['numeric']>;
  royalty_points_numerator?: InputMaybe<Scalars['numeric']>;
  supply?: InputMaybe<Scalars['numeric']>;
  token_data_id_hash?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
  uri_mutable?: InputMaybe<Scalars['Boolean']>;
};

/** columns and relationships of "token_ownerships" */
export type Token_Ownerships = {
  __typename?: 'token_ownerships';
  amount: Scalars['numeric'];
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  name: Scalars['String'];
  owner_address?: Maybe<Scalars['String']>;
  property_version: Scalars['numeric'];
  table_handle: Scalars['String'];
  table_type?: Maybe<Scalars['String']>;
  token_data_id_hash: Scalars['String'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
};

/** Boolean expression to filter rows from the table "token_ownerships". All fields are combined with a logical 'AND'. */
export type Token_Ownerships_Bool_Exp = {
  _and?: InputMaybe<Array<Token_Ownerships_Bool_Exp>>;
  _not?: InputMaybe<Token_Ownerships_Bool_Exp>;
  _or?: InputMaybe<Array<Token_Ownerships_Bool_Exp>>;
  amount?: InputMaybe<Numeric_Comparison_Exp>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  owner_address?: InputMaybe<String_Comparison_Exp>;
  property_version?: InputMaybe<Numeric_Comparison_Exp>;
  table_handle?: InputMaybe<String_Comparison_Exp>;
  table_type?: InputMaybe<String_Comparison_Exp>;
  token_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "token_ownerships". */
export type Token_Ownerships_Order_By = {
  amount?: InputMaybe<Order_By>;
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  owner_address?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  table_handle?: InputMaybe<Order_By>;
  table_type?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "token_ownerships" */
export enum Token_Ownerships_Select_Column {
  /** column name */
  Amount = 'amount',
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  Name = 'name',
  /** column name */
  OwnerAddress = 'owner_address',
  /** column name */
  PropertyVersion = 'property_version',
  /** column name */
  TableHandle = 'table_handle',
  /** column name */
  TableType = 'table_type',
  /** column name */
  TokenDataIdHash = 'token_data_id_hash',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** Streaming cursor of the table "token_ownerships" */
export type Token_Ownerships_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Token_Ownerships_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Token_Ownerships_Stream_Cursor_Value_Input = {
  amount?: InputMaybe<Scalars['numeric']>;
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  name?: InputMaybe<Scalars['String']>;
  owner_address?: InputMaybe<Scalars['String']>;
  property_version?: InputMaybe<Scalars['numeric']>;
  table_handle?: InputMaybe<Scalars['String']>;
  table_type?: InputMaybe<Scalars['String']>;
  token_data_id_hash?: InputMaybe<Scalars['String']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "tokens" */
export type Tokens = {
  __typename?: 'tokens';
  collection_data_id_hash: Scalars['String'];
  collection_name: Scalars['String'];
  creator_address: Scalars['String'];
  name: Scalars['String'];
  property_version: Scalars['numeric'];
  token_data_id_hash: Scalars['String'];
  token_properties: Scalars['jsonb'];
  transaction_timestamp: Scalars['timestamp'];
  transaction_version: Scalars['bigint'];
};


/** columns and relationships of "tokens" */
export type TokensToken_PropertiesArgs = {
  path?: InputMaybe<Scalars['String']>;
};

/** Boolean expression to filter rows from the table "tokens". All fields are combined with a logical 'AND'. */
export type Tokens_Bool_Exp = {
  _and?: InputMaybe<Array<Tokens_Bool_Exp>>;
  _not?: InputMaybe<Tokens_Bool_Exp>;
  _or?: InputMaybe<Array<Tokens_Bool_Exp>>;
  collection_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  collection_name?: InputMaybe<String_Comparison_Exp>;
  creator_address?: InputMaybe<String_Comparison_Exp>;
  name?: InputMaybe<String_Comparison_Exp>;
  property_version?: InputMaybe<Numeric_Comparison_Exp>;
  token_data_id_hash?: InputMaybe<String_Comparison_Exp>;
  token_properties?: InputMaybe<Jsonb_Comparison_Exp>;
  transaction_timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  transaction_version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "tokens". */
export type Tokens_Order_By = {
  collection_data_id_hash?: InputMaybe<Order_By>;
  collection_name?: InputMaybe<Order_By>;
  creator_address?: InputMaybe<Order_By>;
  name?: InputMaybe<Order_By>;
  property_version?: InputMaybe<Order_By>;
  token_data_id_hash?: InputMaybe<Order_By>;
  token_properties?: InputMaybe<Order_By>;
  transaction_timestamp?: InputMaybe<Order_By>;
  transaction_version?: InputMaybe<Order_By>;
};

/** select columns of table "tokens" */
export enum Tokens_Select_Column {
  /** column name */
  CollectionDataIdHash = 'collection_data_id_hash',
  /** column name */
  CollectionName = 'collection_name',
  /** column name */
  CreatorAddress = 'creator_address',
  /** column name */
  Name = 'name',
  /** column name */
  PropertyVersion = 'property_version',
  /** column name */
  TokenDataIdHash = 'token_data_id_hash',
  /** column name */
  TokenProperties = 'token_properties',
  /** column name */
  TransactionTimestamp = 'transaction_timestamp',
  /** column name */
  TransactionVersion = 'transaction_version'
}

/** Streaming cursor of the table "tokens" */
export type Tokens_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: Tokens_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type Tokens_Stream_Cursor_Value_Input = {
  collection_data_id_hash?: InputMaybe<Scalars['String']>;
  collection_name?: InputMaybe<Scalars['String']>;
  creator_address?: InputMaybe<Scalars['String']>;
  name?: InputMaybe<Scalars['String']>;
  property_version?: InputMaybe<Scalars['numeric']>;
  token_data_id_hash?: InputMaybe<Scalars['String']>;
  token_properties?: InputMaybe<Scalars['jsonb']>;
  transaction_timestamp?: InputMaybe<Scalars['timestamp']>;
  transaction_version?: InputMaybe<Scalars['bigint']>;
};

/** columns and relationships of "user_transactions" */
export type User_Transactions = {
  __typename?: 'user_transactions';
  block_height: Scalars['bigint'];
  entry_function_id_str: Scalars['String'];
  epoch: Scalars['bigint'];
  expiration_timestamp_secs: Scalars['timestamp'];
  gas_unit_price: Scalars['numeric'];
  max_gas_amount: Scalars['numeric'];
  parent_signature_type: Scalars['String'];
  sender: Scalars['String'];
  sequence_number: Scalars['bigint'];
  timestamp: Scalars['timestamp'];
  version: Scalars['bigint'];
};

/** Boolean expression to filter rows from the table "user_transactions". All fields are combined with a logical 'AND'. */
export type User_Transactions_Bool_Exp = {
  _and?: InputMaybe<Array<User_Transactions_Bool_Exp>>;
  _not?: InputMaybe<User_Transactions_Bool_Exp>;
  _or?: InputMaybe<Array<User_Transactions_Bool_Exp>>;
  block_height?: InputMaybe<Bigint_Comparison_Exp>;
  entry_function_id_str?: InputMaybe<String_Comparison_Exp>;
  epoch?: InputMaybe<Bigint_Comparison_Exp>;
  expiration_timestamp_secs?: InputMaybe<Timestamp_Comparison_Exp>;
  gas_unit_price?: InputMaybe<Numeric_Comparison_Exp>;
  max_gas_amount?: InputMaybe<Numeric_Comparison_Exp>;
  parent_signature_type?: InputMaybe<String_Comparison_Exp>;
  sender?: InputMaybe<String_Comparison_Exp>;
  sequence_number?: InputMaybe<Bigint_Comparison_Exp>;
  timestamp?: InputMaybe<Timestamp_Comparison_Exp>;
  version?: InputMaybe<Bigint_Comparison_Exp>;
};

/** Ordering options when selecting data from "user_transactions". */
export type User_Transactions_Order_By = {
  block_height?: InputMaybe<Order_By>;
  entry_function_id_str?: InputMaybe<Order_By>;
  epoch?: InputMaybe<Order_By>;
  expiration_timestamp_secs?: InputMaybe<Order_By>;
  gas_unit_price?: InputMaybe<Order_By>;
  max_gas_amount?: InputMaybe<Order_By>;
  parent_signature_type?: InputMaybe<Order_By>;
  sender?: InputMaybe<Order_By>;
  sequence_number?: InputMaybe<Order_By>;
  timestamp?: InputMaybe<Order_By>;
  version?: InputMaybe<Order_By>;
};

/** select columns of table "user_transactions" */
export enum User_Transactions_Select_Column {
  /** column name */
  BlockHeight = 'block_height',
  /** column name */
  EntryFunctionIdStr = 'entry_function_id_str',
  /** column name */
  Epoch = 'epoch',
  /** column name */
  ExpirationTimestampSecs = 'expiration_timestamp_secs',
  /** column name */
  GasUnitPrice = 'gas_unit_price',
  /** column name */
  MaxGasAmount = 'max_gas_amount',
  /** column name */
  ParentSignatureType = 'parent_signature_type',
  /** column name */
  Sender = 'sender',
  /** column name */
  SequenceNumber = 'sequence_number',
  /** column name */
  Timestamp = 'timestamp',
  /** column name */
  Version = 'version'
}

/** Streaming cursor of the table "user_transactions" */
export type User_Transactions_Stream_Cursor_Input = {
  /** Stream column input with initial value */
  initial_value: User_Transactions_Stream_Cursor_Value_Input;
  /** cursor ordering */
  ordering?: InputMaybe<Cursor_Ordering>;
};

/** Initial value of the column from where the streaming should start */
export type User_Transactions_Stream_Cursor_Value_Input = {
  block_height?: InputMaybe<Scalars['bigint']>;
  entry_function_id_str?: InputMaybe<Scalars['String']>;
  epoch?: InputMaybe<Scalars['bigint']>;
  expiration_timestamp_secs?: InputMaybe<Scalars['timestamp']>;
  gas_unit_price?: InputMaybe<Scalars['numeric']>;
  max_gas_amount?: InputMaybe<Scalars['numeric']>;
  parent_signature_type?: InputMaybe<Scalars['String']>;
  sender?: InputMaybe<Scalars['String']>;
  sequence_number?: InputMaybe<Scalars['bigint']>;
  timestamp?: InputMaybe<Scalars['timestamp']>;
  version?: InputMaybe<Scalars['bigint']>;
};

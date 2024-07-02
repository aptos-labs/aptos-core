
<a id="0x3_token"></a>

# Module `0x3::token`

This module provides the foundation for Tokens.
Checkout our developer doc on our token standard https://aptos.dev/standards


-  [Struct `Token`](#0x3_token_Token)
-  [Struct `TokenId`](#0x3_token_TokenId)
-  [Struct `TokenDataId`](#0x3_token_TokenDataId)
-  [Struct `TokenData`](#0x3_token_TokenData)
-  [Struct `Royalty`](#0x3_token_Royalty)
-  [Struct `TokenMutabilityConfig`](#0x3_token_TokenMutabilityConfig)
-  [Resource `TokenStore`](#0x3_token_TokenStore)
-  [Struct `CollectionMutabilityConfig`](#0x3_token_CollectionMutabilityConfig)
-  [Resource `Collections`](#0x3_token_Collections)
-  [Struct `CollectionData`](#0x3_token_CollectionData)
-  [Struct `WithdrawCapability`](#0x3_token_WithdrawCapability)
-  [Struct `DepositEvent`](#0x3_token_DepositEvent)
-  [Struct `Deposit`](#0x3_token_Deposit)
-  [Struct `WithdrawEvent`](#0x3_token_WithdrawEvent)
-  [Struct `Withdraw`](#0x3_token_Withdraw)
-  [Struct `CreateTokenDataEvent`](#0x3_token_CreateTokenDataEvent)
-  [Struct `CreateTokenData`](#0x3_token_CreateTokenData)
-  [Struct `MintTokenEvent`](#0x3_token_MintTokenEvent)
-  [Struct `MintToken`](#0x3_token_MintToken)
-  [Struct `BurnTokenEvent`](#0x3_token_BurnTokenEvent)
-  [Struct `BurnToken`](#0x3_token_BurnToken)
-  [Struct `MutateTokenPropertyMapEvent`](#0x3_token_MutateTokenPropertyMapEvent)
-  [Struct `MutateTokenPropertyMap`](#0x3_token_MutateTokenPropertyMap)
-  [Struct `CreateCollectionEvent`](#0x3_token_CreateCollectionEvent)
-  [Struct `CreateCollection`](#0x3_token_CreateCollection)
-  [Constants](#@Constants_0)
-  [Function `create_collection_script`](#0x3_token_create_collection_script)
-  [Function `create_token_script`](#0x3_token_create_token_script)
-  [Function `mint_script`](#0x3_token_mint_script)
-  [Function `mutate_token_properties`](#0x3_token_mutate_token_properties)
-  [Function `direct_transfer_script`](#0x3_token_direct_transfer_script)
-  [Function `opt_in_direct_transfer`](#0x3_token_opt_in_direct_transfer)
-  [Function `transfer_with_opt_in`](#0x3_token_transfer_with_opt_in)
-  [Function `burn_by_creator`](#0x3_token_burn_by_creator)
-  [Function `burn`](#0x3_token_burn)
-  [Function `mutate_collection_description`](#0x3_token_mutate_collection_description)
-  [Function `mutate_collection_uri`](#0x3_token_mutate_collection_uri)
-  [Function `mutate_collection_maximum`](#0x3_token_mutate_collection_maximum)
-  [Function `mutate_tokendata_maximum`](#0x3_token_mutate_tokendata_maximum)
-  [Function `mutate_tokendata_uri`](#0x3_token_mutate_tokendata_uri)
-  [Function `mutate_tokendata_royalty`](#0x3_token_mutate_tokendata_royalty)
-  [Function `mutate_tokendata_description`](#0x3_token_mutate_tokendata_description)
-  [Function `mutate_tokendata_property`](#0x3_token_mutate_tokendata_property)
-  [Function `mutate_one_token`](#0x3_token_mutate_one_token)
-  [Function `create_royalty`](#0x3_token_create_royalty)
-  [Function `deposit_token`](#0x3_token_deposit_token)
-  [Function `direct_deposit_with_opt_in`](#0x3_token_direct_deposit_with_opt_in)
-  [Function `direct_transfer`](#0x3_token_direct_transfer)
-  [Function `initialize_token_store`](#0x3_token_initialize_token_store)
-  [Function `merge`](#0x3_token_merge)
-  [Function `split`](#0x3_token_split)
-  [Function `token_id`](#0x3_token_token_id)
-  [Function `transfer`](#0x3_token_transfer)
-  [Function `create_withdraw_capability`](#0x3_token_create_withdraw_capability)
-  [Function `withdraw_with_capability`](#0x3_token_withdraw_with_capability)
-  [Function `partial_withdraw_with_capability`](#0x3_token_partial_withdraw_with_capability)
-  [Function `withdraw_token`](#0x3_token_withdraw_token)
-  [Function `create_collection`](#0x3_token_create_collection)
-  [Function `check_collection_exists`](#0x3_token_check_collection_exists)
-  [Function `check_tokendata_exists`](#0x3_token_check_tokendata_exists)
-  [Function `create_tokendata`](#0x3_token_create_tokendata)
-  [Function `get_collection_supply`](#0x3_token_get_collection_supply)
-  [Function `get_collection_description`](#0x3_token_get_collection_description)
-  [Function `get_collection_uri`](#0x3_token_get_collection_uri)
-  [Function `get_collection_maximum`](#0x3_token_get_collection_maximum)
-  [Function `get_token_supply`](#0x3_token_get_token_supply)
-  [Function `get_tokendata_largest_property_version`](#0x3_token_get_tokendata_largest_property_version)
-  [Function `get_token_id`](#0x3_token_get_token_id)
-  [Function `get_direct_transfer`](#0x3_token_get_direct_transfer)
-  [Function `create_token_mutability_config`](#0x3_token_create_token_mutability_config)
-  [Function `create_collection_mutability_config`](#0x3_token_create_collection_mutability_config)
-  [Function `mint_token`](#0x3_token_mint_token)
-  [Function `mint_token_to`](#0x3_token_mint_token_to)
-  [Function `create_token_id`](#0x3_token_create_token_id)
-  [Function `create_token_data_id`](#0x3_token_create_token_data_id)
-  [Function `create_token_id_raw`](#0x3_token_create_token_id_raw)
-  [Function `balance_of`](#0x3_token_balance_of)
-  [Function `has_token_store`](#0x3_token_has_token_store)
-  [Function `get_royalty`](#0x3_token_get_royalty)
-  [Function `get_royalty_numerator`](#0x3_token_get_royalty_numerator)
-  [Function `get_royalty_denominator`](#0x3_token_get_royalty_denominator)
-  [Function `get_royalty_payee`](#0x3_token_get_royalty_payee)
-  [Function `get_token_amount`](#0x3_token_get_token_amount)
-  [Function `get_token_id_fields`](#0x3_token_get_token_id_fields)
-  [Function `get_token_data_id_fields`](#0x3_token_get_token_data_id_fields)
-  [Function `get_property_map`](#0x3_token_get_property_map)
-  [Function `get_tokendata_maximum`](#0x3_token_get_tokendata_maximum)
-  [Function `get_tokendata_uri`](#0x3_token_get_tokendata_uri)
-  [Function `get_tokendata_description`](#0x3_token_get_tokendata_description)
-  [Function `get_tokendata_royalty`](#0x3_token_get_tokendata_royalty)
-  [Function `get_tokendata_id`](#0x3_token_get_tokendata_id)
-  [Function `get_tokendata_mutability_config`](#0x3_token_get_tokendata_mutability_config)
-  [Function `get_token_mutability_maximum`](#0x3_token_get_token_mutability_maximum)
-  [Function `get_token_mutability_royalty`](#0x3_token_get_token_mutability_royalty)
-  [Function `get_token_mutability_uri`](#0x3_token_get_token_mutability_uri)
-  [Function `get_token_mutability_description`](#0x3_token_get_token_mutability_description)
-  [Function `get_token_mutability_default_properties`](#0x3_token_get_token_mutability_default_properties)
-  [Function `get_collection_mutability_config`](#0x3_token_get_collection_mutability_config)
-  [Function `get_collection_mutability_description`](#0x3_token_get_collection_mutability_description)
-  [Function `get_collection_mutability_uri`](#0x3_token_get_collection_mutability_uri)
-  [Function `get_collection_mutability_maximum`](#0x3_token_get_collection_mutability_maximum)
-  [Function `destroy_token_data`](#0x3_token_destroy_token_data)
-  [Function `destroy_collection_data`](#0x3_token_destroy_collection_data)
-  [Function `withdraw_with_event_internal`](#0x3_token_withdraw_with_event_internal)
-  [Function `update_token_property_internal`](#0x3_token_update_token_property_internal)
-  [Function `direct_deposit`](#0x3_token_direct_deposit)
-  [Function `assert_collection_exists`](#0x3_token_assert_collection_exists)
-  [Function `assert_tokendata_exists`](#0x3_token_assert_tokendata_exists)
-  [Function `assert_non_standard_reserved_property`](#0x3_token_assert_non_standard_reserved_property)
-  [Function `initialize_token_script`](#0x3_token_initialize_token_script)
-  [Function `initialize_token`](#0x3_token_initialize_token)
-  [Specification](#@Specification_1)
    -  [Function `create_collection_script`](#@Specification_1_create_collection_script)
    -  [Function `create_token_script`](#@Specification_1_create_token_script)
    -  [Function `mint_script`](#@Specification_1_mint_script)
    -  [Function `mutate_token_properties`](#@Specification_1_mutate_token_properties)
    -  [Function `direct_transfer_script`](#@Specification_1_direct_transfer_script)
    -  [Function `opt_in_direct_transfer`](#@Specification_1_opt_in_direct_transfer)
    -  [Function `transfer_with_opt_in`](#@Specification_1_transfer_with_opt_in)
    -  [Function `burn_by_creator`](#@Specification_1_burn_by_creator)
    -  [Function `burn`](#@Specification_1_burn)
    -  [Function `mutate_collection_description`](#@Specification_1_mutate_collection_description)
    -  [Function `mutate_collection_uri`](#@Specification_1_mutate_collection_uri)
    -  [Function `mutate_collection_maximum`](#@Specification_1_mutate_collection_maximum)
    -  [Function `mutate_tokendata_maximum`](#@Specification_1_mutate_tokendata_maximum)
    -  [Function `mutate_tokendata_uri`](#@Specification_1_mutate_tokendata_uri)
    -  [Function `mutate_tokendata_royalty`](#@Specification_1_mutate_tokendata_royalty)
    -  [Function `mutate_tokendata_description`](#@Specification_1_mutate_tokendata_description)
    -  [Function `mutate_tokendata_property`](#@Specification_1_mutate_tokendata_property)
    -  [Function `mutate_one_token`](#@Specification_1_mutate_one_token)
    -  [Function `create_royalty`](#@Specification_1_create_royalty)
    -  [Function `deposit_token`](#@Specification_1_deposit_token)
    -  [Function `direct_deposit_with_opt_in`](#@Specification_1_direct_deposit_with_opt_in)
    -  [Function `direct_transfer`](#@Specification_1_direct_transfer)
    -  [Function `initialize_token_store`](#@Specification_1_initialize_token_store)
    -  [Function `merge`](#@Specification_1_merge)
    -  [Function `split`](#@Specification_1_split)
    -  [Function `transfer`](#@Specification_1_transfer)
    -  [Function `withdraw_with_capability`](#@Specification_1_withdraw_with_capability)
    -  [Function `partial_withdraw_with_capability`](#@Specification_1_partial_withdraw_with_capability)
    -  [Function `withdraw_token`](#@Specification_1_withdraw_token)
    -  [Function `create_collection`](#@Specification_1_create_collection)
    -  [Function `check_collection_exists`](#@Specification_1_check_collection_exists)
    -  [Function `check_tokendata_exists`](#@Specification_1_check_tokendata_exists)
    -  [Function `create_tokendata`](#@Specification_1_create_tokendata)
    -  [Function `get_collection_supply`](#@Specification_1_get_collection_supply)
    -  [Function `get_collection_description`](#@Specification_1_get_collection_description)
    -  [Function `get_collection_uri`](#@Specification_1_get_collection_uri)
    -  [Function `get_collection_maximum`](#@Specification_1_get_collection_maximum)
    -  [Function `get_token_supply`](#@Specification_1_get_token_supply)
    -  [Function `get_tokendata_largest_property_version`](#@Specification_1_get_tokendata_largest_property_version)
    -  [Function `create_token_mutability_config`](#@Specification_1_create_token_mutability_config)
    -  [Function `create_collection_mutability_config`](#@Specification_1_create_collection_mutability_config)
    -  [Function `mint_token`](#@Specification_1_mint_token)
    -  [Function `mint_token_to`](#@Specification_1_mint_token_to)
    -  [Function `create_token_data_id`](#@Specification_1_create_token_data_id)
    -  [Function `create_token_id_raw`](#@Specification_1_create_token_id_raw)
    -  [Function `get_royalty`](#@Specification_1_get_royalty)
    -  [Function `get_property_map`](#@Specification_1_get_property_map)
    -  [Function `get_tokendata_maximum`](#@Specification_1_get_tokendata_maximum)
    -  [Function `get_tokendata_uri`](#@Specification_1_get_tokendata_uri)
    -  [Function `get_tokendata_description`](#@Specification_1_get_tokendata_description)
    -  [Function `get_tokendata_royalty`](#@Specification_1_get_tokendata_royalty)
    -  [Function `get_tokendata_mutability_config`](#@Specification_1_get_tokendata_mutability_config)
    -  [Function `get_collection_mutability_config`](#@Specification_1_get_collection_mutability_config)
    -  [Function `withdraw_with_event_internal`](#@Specification_1_withdraw_with_event_internal)
    -  [Function `update_token_property_internal`](#@Specification_1_update_token_property_internal)
    -  [Function `direct_deposit`](#@Specification_1_direct_deposit)
    -  [Function `assert_collection_exists`](#@Specification_1_assert_collection_exists)
    -  [Function `assert_tokendata_exists`](#@Specification_1_assert_tokendata_exists)
    -  [Function `assert_non_standard_reserved_property`](#@Specification_1_assert_non_standard_reserved_property)
    -  [Function `initialize_token_script`](#@Specification_1_initialize_token_script)
    -  [Function `initialize_token`](#@Specification_1_initialize_token)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="property_map.md#0x3_property_map">0x3::property_map</a>;<br /><b>use</b> <a href="token_event_store.md#0x3_token_event_store">0x3::token_event_store</a>;<br /></code></pre>



<a id="0x3_token_Token"></a>

## Struct `Token`



<pre><code><b>struct</b> <a href="token.md#0x3_token_Token">Token</a> <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>
 the amount of tokens. Only property_version &#61; 0 can have a value bigger than 1.
</dd>
<dt>
<code>token_properties: <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a></code>
</dt>
<dd>
 The properties with this token.
 when property_version &#61; 0, the token_properties are the same as default_properties in TokenData, we don&apos;t store it.
 when the property_map mutates, a new property_version is assigned to the token.
</dd>
</dl>


</details>

<a id="0x3_token_TokenId"></a>

## Struct `TokenId`

global unique identifier of a token


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenId">TokenId</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>
 the id to the common token data shared by token with different property_version
</dd>
<dt>
<code>property_version: u64</code>
</dt>
<dd>
 The version of the property map; when a fungible token is mutated, a new property version is created and assigned to the token to make it an NFT
</dd>
</dl>


</details>

<a id="0x3_token_TokenDataId"></a>

## Struct `TokenDataId`

globally unique identifier of tokendata


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>
 The address of the creator, eg: 0xcafe
</dd>
<dt>
<code>collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The name of collection; this is unique under the same account, eg: &quot;Aptos Animal Collection&quot;
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The name of the token; this is the same as the name field of TokenData
</dd>
</dl>


</details>

<a id="0x3_token_TokenData"></a>

## Struct `TokenData`

The shared TokenData by tokens with different property_version


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenData">TokenData</a> <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>maximum: u64</code>
</dt>
<dd>
 The maximal number of tokens that can be minted under this TokenData; if the maximum is 0, there is no limit
</dd>
<dt>
<code>largest_property_version: u64</code>
</dt>
<dd>
 The current largest property version of all tokens with this TokenData
</dd>
<dt>
<code>supply: u64</code>
</dt>
<dd>
 The number of tokens with this TokenData. Supply is only tracked for the limited token whose maximum is not 0
</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off&#45;chain storage; the URL length should be less than 512 characters, eg: https://arweave.net/Fmmn4ul&#45;7Mv6vzm7JwE69O&#45;I&#45;vd6Bz2QriJO1niwCh4
</dd>
<dt>
<code>royalty: <a href="token.md#0x3_token_Royalty">token::Royalty</a></code>
</dt>
<dd>
 The denominator and numerator for calculating the royalty fee; it also contains payee account address for depositing the Royalty
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The name of the token, which should be unique within the collection; the length of name should be smaller than 128, characters, eg: &quot;Aptos Animal #1234&quot;
</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Describes this Token
</dd>
<dt>
<code>default_properties: <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a></code>
</dt>
<dd>
 The properties are stored in the TokenData that are shared by all tokens
</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a></code>
</dt>
<dd>
 Control the TokenData field mutability
</dd>
</dl>


</details>

<a id="0x3_token_Royalty"></a>

## Struct `Royalty`

The royalty of a token


<pre><code><b>struct</b> <a href="token.md#0x3_token_Royalty">Royalty</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>royalty_points_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>payee_address: <b>address</b></code>
</dt>
<dd>
 if the token is jointly owned by multiple creators, the group of creators should create a shared account.
 the payee_address will be the shared account address.
</dd>
</dl>


</details>

<a id="0x3_token_TokenMutabilityConfig"></a>

## Struct `TokenMutabilityConfig`

This config specifies which fields in the TokenData are mutable


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>maximum: bool</code>
</dt>
<dd>
 control if the token maximum is mutable
</dd>
<dt>
<code>uri: bool</code>
</dt>
<dd>
 control if the token uri is mutable
</dd>
<dt>
<code>royalty: bool</code>
</dt>
<dd>
 control if the token royalty is mutable
</dd>
<dt>
<code>description: bool</code>
</dt>
<dd>
 control if the token description is mutable
</dd>
<dt>
<code>properties: bool</code>
</dt>
<dd>
 control if the property map is mutable
</dd>
</dl>


</details>

<a id="0x3_token_TokenStore"></a>

## Resource `TokenStore`

Represents token resources owned by token owner


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tokens: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token.md#0x3_token_TokenId">token::TokenId</a>, <a href="token.md#0x3_token_Token">token::Token</a>&gt;</code>
</dt>
<dd>
 the tokens owned by a token owner
</dd>
<dt>
<code>direct_transfer: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_DepositEvent">token::DepositEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_WithdrawEvent">token::WithdrawEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">token::BurnTokenEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mutate_token_property_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">token::MutateTokenPropertyMapEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CollectionMutabilityConfig"></a>

## Struct `CollectionMutabilityConfig`

This config specifies which fields in the Collection are mutable


<pre><code><b>struct</b> <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>description: bool</code>
</dt>
<dd>
 control if description is mutable
</dd>
<dt>
<code>uri: bool</code>
</dt>
<dd>
 control if uri is mutable
</dd>
<dt>
<code>maximum: bool</code>
</dt>
<dd>
 control if collection maxium is mutable
</dd>
</dl>


</details>

<a id="0x3_token_Collections"></a>

## Resource `Collections`

Represent collection and token metadata for a creator


<pre><code><b>struct</b> <a href="token.md#0x3_token_Collections">Collections</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token_CollectionData">token::CollectionData</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>token_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, <a href="token.md#0x3_token_TokenData">token::TokenData</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_collection_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_CreateCollectionEvent">token::CreateCollectionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_token_data_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_CreateTokenDataEvent">token::CreateTokenDataEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mint_token_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">token::MintTokenEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CollectionData"></a>

## Struct `CollectionData`

Represent the collection metadata


<pre><code><b>struct</b> <a href="token.md#0x3_token_CollectionData">CollectionData</a> <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 A description for the token collection Eg: &quot;Aptos Toad Overload&quot;
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The collection name, which should be unique among all collections by the creator; the name should also be smaller than 128 characters, eg: &quot;Animal Collection&quot;
</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The URI for the collection; its length should be smaller than 512 characters
</dd>
<dt>
<code>supply: u64</code>
</dt>
<dd>
 The number of different TokenData entries in this collection
</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>
 If maximal is a non&#45;zero value, the number of created TokenData entries should be smaller or equal to this maximum
 If maximal is 0, Aptos doesn&apos;t track the supply of this collection, and there is no limit
</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a></code>
</dt>
<dd>
 control which collectionData field is mutable
</dd>
</dl>


</details>

<a id="0x3_token_WithdrawCapability"></a>

## Struct `WithdrawCapability`

capability to withdraw without signer, this struct should be non&#45;copyable


<pre><code><b>struct</b> <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_sec: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_DepositEvent"></a>

## Struct `DepositEvent`

Set of data sent to the event stream during a receive


<pre><code><b>struct</b> <a href="token.md#0x3_token_DepositEvent">DepositEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_Deposit"></a>

## Struct `Deposit`

Set of data sent to the event stream during a receive


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x3_token_Deposit">Deposit</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_WithdrawEvent"></a>

## Struct `WithdrawEvent`

Set of data sent to the event stream during a withdrawal


<pre><code><b>struct</b> <a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_Withdraw"></a>

## Struct `Withdraw`

Set of data sent to the event stream during a withdrawal


<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x3_token_Withdraw">Withdraw</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateTokenDataEvent"></a>

## Struct `CreateTokenDataEvent`

token creation event id of token created


<pre><code><b>struct</b> <a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateTokenData"></a>

## Struct `CreateTokenData`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x3_token_CreateTokenData">CreateTokenData</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MintTokenEvent"></a>

## Struct `MintTokenEvent`

mint token event. This event triggered when creator adds more supply to existing token


<pre><code><b>struct</b> <a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MintToken"></a>

## Struct `MintToken`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x3_token_MintToken">MintToken</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_BurnTokenEvent"></a>

## Struct `BurnTokenEvent`



<pre><code><b>struct</b> <a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_BurnToken"></a>

## Struct `BurnToken`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x3_token_BurnToken">BurnToken</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MutateTokenPropertyMapEvent"></a>

## Struct `MutateTokenPropertyMapEvent`



<pre><code><b>struct</b> <a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MutateTokenPropertyMap"></a>

## Struct `MutateTokenPropertyMap`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x3_token_MutateTokenPropertyMap">MutateTokenPropertyMap</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateCollectionEvent"></a>

## Struct `CreateCollectionEvent`

create collection event with creator address and collection name


<pre><code><b>struct</b> <a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateCollection"></a>

## Struct `CreateCollection`



<pre><code>&#35;[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="token.md#0x3_token_CreateCollection">CreateCollection</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_token_EINSUFFICIENT_BALANCE"></a>

Insufficient token balance


<pre><code><b>const</b> <a href="token.md#0x3_token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x3_token_EURI_TOO_LONG"></a>

The URI is too long


<pre><code><b>const</b> <a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>: u64 &#61; 27;<br /></code></pre>



<a id="0x3_token_MAX_URI_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>: u64 &#61; 512;<br /></code></pre>



<a id="0x3_token_BURNABLE_BY_CREATOR"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 67, 82, 69, 65, 84, 79, 82];<br /></code></pre>



<a id="0x3_token_BURNABLE_BY_OWNER"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 79, 87, 78, 69, 82];<br /></code></pre>



<a id="0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND">COLLECTION_DESCRIPTION_MUTABLE_IND</a>: u64 &#61; 0;<br /></code></pre>



<a id="0x3_token_COLLECTION_MAX_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_COLLECTION_MAX_MUTABLE_IND">COLLECTION_MAX_MUTABLE_IND</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x3_token_COLLECTION_URI_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_COLLECTION_URI_MUTABLE_IND">COLLECTION_URI_MUTABLE_IND</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x3_token_EALREADY_HAS_BALANCE"></a>

The token has balance and cannot be initialized


<pre><code><b>const</b> <a href="token.md#0x3_token_EALREADY_HAS_BALANCE">EALREADY_HAS_BALANCE</a>: u64 &#61; 0;<br /></code></pre>



<a id="0x3_token_ECANNOT_UPDATE_RESERVED_PROPERTY"></a>

Reserved fields for token contract
Cannot be updated by user


<pre><code><b>const</b> <a href="token.md#0x3_token_ECANNOT_UPDATE_RESERVED_PROPERTY">ECANNOT_UPDATE_RESERVED_PROPERTY</a>: u64 &#61; 32;<br /></code></pre>



<a id="0x3_token_ECOLLECTIONS_NOT_PUBLISHED"></a>

There isn&apos;t any collection under this account


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x3_token_ECOLLECTION_ALREADY_EXISTS"></a>

The collection already exists


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTION_ALREADY_EXISTS">ECOLLECTION_ALREADY_EXISTS</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x3_token_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is too long


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>: u64 &#61; 25;<br /></code></pre>



<a id="0x3_token_ECOLLECTION_NOT_PUBLISHED"></a>

Cannot find collection in creator&apos;s account


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM"></a>

Exceeds the collection&apos;s maximal number of token_data


<pre><code><b>const</b> <a href="token.md#0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM">ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x3_token_ECREATOR_CANNOT_BURN_TOKEN"></a>

Token is not burnable by creator


<pre><code><b>const</b> <a href="token.md#0x3_token_ECREATOR_CANNOT_BURN_TOKEN">ECREATOR_CANNOT_BURN_TOKEN</a>: u64 &#61; 31;<br /></code></pre>



<a id="0x3_token_EFIELD_NOT_MUTABLE"></a>

The field is not mutable


<pre><code><b>const</b> <a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT"></a>

Withdraw capability doesn&apos;t have sufficient amount


<pre><code><b>const</b> <a href="token.md#0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT">EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT</a>: u64 &#61; 38;<br /></code></pre>



<a id="0x3_token_EINVALID_MAXIMUM"></a>

Collection or tokendata maximum must be larger than supply


<pre><code><b>const</b> <a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>: u64 &#61; 36;<br /></code></pre>



<a id="0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR"></a>

Royalty invalid if the numerator is larger than the denominator


<pre><code><b>const</b> <a href="token.md#0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR">EINVALID_ROYALTY_NUMERATOR_DENOMINATOR</a>: u64 &#61; 34;<br /></code></pre>



<a id="0x3_token_EINVALID_TOKEN_MERGE"></a>

Cannot merge the two tokens with different token id


<pre><code><b>const</b> <a href="token.md#0x3_token_EINVALID_TOKEN_MERGE">EINVALID_TOKEN_MERGE</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM"></a>

Exceed the token data maximal allowed


<pre><code><b>const</b> <a href="token.md#0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM">EMINT_WOULD_EXCEED_TOKEN_MAXIMUM</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x3_token_ENFT_NAME_TOO_LONG"></a>

The NFT name is too long


<pre><code><b>const</b> <a href="token.md#0x3_token_ENFT_NAME_TOO_LONG">ENFT_NAME_TOO_LONG</a>: u64 &#61; 26;<br /></code></pre>



<a id="0x3_token_ENFT_NOT_SPLITABLE"></a>

Cannot split a token that only has 1 amount


<pre><code><b>const</b> <a href="token.md#0x3_token_ENFT_NOT_SPLITABLE">ENFT_NOT_SPLITABLE</a>: u64 &#61; 18;<br /></code></pre>



<a id="0x3_token_ENO_BURN_CAPABILITY"></a>

No burn capability


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_BURN_CAPABILITY">ENO_BURN_CAPABILITY</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot burn 0 Token


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT">ENO_BURN_TOKEN_WITH_ZERO_AMOUNT</a>: u64 &#61; 29;<br /></code></pre>



<a id="0x3_token_ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot deposit a Token with 0 amount


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT">ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT</a>: u64 &#61; 28;<br /></code></pre>



<a id="0x3_token_ENO_MINT_CAPABILITY"></a>

No mint capability


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>: u64 &#61; 19;<br /></code></pre>



<a id="0x3_token_ENO_MUTATE_CAPABILITY"></a>

Not authorized to mutate


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x3_token_ENO_TOKEN_IN_TOKEN_STORE"></a>

Token not in the token store


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_TOKEN_IN_TOKEN_STORE">ENO_TOKEN_IN_TOKEN_STORE</a>: u64 &#61; 15;<br /></code></pre>



<a id="0x3_token_EOWNER_CANNOT_BURN_TOKEN"></a>

Token is not burnable by owner


<pre><code><b>const</b> <a href="token.md#0x3_token_EOWNER_CANNOT_BURN_TOKEN">EOWNER_CANNOT_BURN_TOKEN</a>: u64 &#61; 30;<br /></code></pre>



<a id="0x3_token_EPROPERTY_RESERVED_BY_STANDARD"></a>

The property is reserved by token standard


<pre><code><b>const</b> <a href="token.md#0x3_token_EPROPERTY_RESERVED_BY_STANDARD">EPROPERTY_RESERVED_BY_STANDARD</a>: u64 &#61; 40;<br /></code></pre>



<a id="0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST"></a>

Royalty payee account does not exist


<pre><code><b>const</b> <a href="token.md#0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST">EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST</a>: u64 &#61; 35;<br /></code></pre>



<a id="0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT"></a>

TOKEN with 0 amount is not allowed


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT">ETOKEN_CANNOT_HAVE_ZERO_AMOUNT</a>: u64 &#61; 33;<br /></code></pre>



<a id="0x3_token_ETOKEN_DATA_ALREADY_EXISTS"></a>

TokenData already exists


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_DATA_ALREADY_EXISTS">ETOKEN_DATA_ALREADY_EXISTS</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x3_token_ETOKEN_DATA_NOT_PUBLISHED"></a>

TokenData not published


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH"></a>

Token Properties count doesn&apos;t match


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH">ETOKEN_PROPERTIES_COUNT_NOT_MATCH</a>: u64 &#61; 37;<br /></code></pre>



<a id="0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT"></a>

Cannot split token to an amount larger than its amount


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT">ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x3_token_ETOKEN_STORE_NOT_PUBLISHED"></a>

TokenStore doesn&apos;t exist


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER"></a>

User didn&apos;t opt&#45;in direct transfer


<pre><code><b>const</b> <a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>: u64 &#61; 16;<br /></code></pre>



<a id="0x3_token_EWITHDRAW_PROOF_EXPIRES"></a>

Withdraw proof expires


<pre><code><b>const</b> <a href="token.md#0x3_token_EWITHDRAW_PROOF_EXPIRES">EWITHDRAW_PROOF_EXPIRES</a>: u64 &#61; 39;<br /></code></pre>



<a id="0x3_token_EWITHDRAW_ZERO"></a>

Cannot withdraw 0 token


<pre><code><b>const</b> <a href="token.md#0x3_token_EWITHDRAW_ZERO">EWITHDRAW_ZERO</a>: u64 &#61; 17;<br /></code></pre>



<a id="0x3_token_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>: u64 &#61; 128;<br /></code></pre>



<a id="0x3_token_MAX_NFT_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>: u64 &#61; 128;<br /></code></pre>



<a id="0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND">TOKEN_DESCRIPTION_MUTABLE_IND</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x3_token_TOKEN_MAX_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_MAX_MUTABLE_IND">TOKEN_MAX_MUTABLE_IND</a>: u64 &#61; 0;<br /></code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 80, 82, 79, 80, 69, 82, 84, 89, 95, 77, 85, 84, 65, 84, 66, 76, 69];<br /></code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE_IND">TOKEN_PROPERTY_MUTABLE_IND</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x3_token_TOKEN_PROPERTY_VALUE_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_PROPERTY_VALUE_MUTABLE_IND">TOKEN_PROPERTY_VALUE_MUTABLE_IND</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x3_token_TOKEN_ROYALTY_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_ROYALTY_MUTABLE_IND">TOKEN_ROYALTY_MUTABLE_IND</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x3_token_TOKEN_URI_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_URI_MUTABLE_IND">TOKEN_URI_MUTABLE_IND</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x3_token_create_collection_script"></a>

## Function `create_collection_script`

create a empty token collection with parameters


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_collection_script">create_collection_script</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_collection_script">create_collection_script</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    name: String,<br />    description: String,<br />    uri: String,<br />    maximum: u64,<br />    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;,<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_create_collection">create_collection</a>(<br />        creator,<br />        name,<br />        description,<br />        uri,<br />        maximum,<br />        mutate_setting<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_token_script"></a>

## Function `create_token_script`

create token with raw inputs


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_token_script">create_token_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, balance: u64, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_token_script">create_token_script</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection: String,<br />    name: String,<br />    description: String,<br />    balance: u64,<br />    maximum: u64,<br />    uri: String,<br />    royalty_payee_address: <b>address</b>,<br />    royalty_points_denominator: u64,<br />    royalty_points_numerator: u64,<br />    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;,<br />    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> token_mut_config &#61; <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(&amp;mutate_setting);<br />    <b>let</b> tokendata_id &#61; <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(<br />        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,<br />        collection,<br />        name,<br />        description,<br />        maximum,<br />        uri,<br />        royalty_payee_address,<br />        royalty_points_denominator,<br />        royalty_points_numerator,<br />        token_mut_config,<br />        property_keys,<br />        property_values,<br />        property_types<br />    );<br /><br />    <a href="token.md#0x3_token_mint_token">mint_token</a>(<br />        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,<br />        tokendata_id,<br />        balance,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mint_script"></a>

## Function `mint_script`

Mint more token from an existing token_data. Mint only adds more token to property_version 0


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mint_script">mint_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mint_script">mint_script</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_data_address: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    amount: u64,<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> token_data_id &#61; <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(<br />        token_data_address,<br />        collection,<br />        name,<br />    );<br />    // only creator of the tokendata can mint more tokens for now<br />    <b>assert</b>!(token_data_id.creator &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>));<br />    <a href="token.md#0x3_token_mint_token">mint_token</a>(<br />        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,<br />        token_data_id,<br />        amount,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_token_properties"></a>

## Function `mutate_token_properties`

mutate the token property and save the new property in TokenStore
if the token property_version is 0, we will create a new property_version per token to generate a new token_id per token
if the token property_version is not 0, we will just update the propertyMap and use the existing token_id (property_version)


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mutate_token_properties">mutate_token_properties</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, amount: u64, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mutate_token_properties">mutate_token_properties</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_owner: <b>address</b>,<br />    creator: <b>address</b>,<br />    collection_name: String,<br />    token_name: String,<br />    token_property_version: u64,<br />    amount: u64,<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>) &#61;&#61; creator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>));<br />    <b>let</b> i &#61; 0;<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(<br />        creator,<br />        collection_name,<br />        token_name,<br />        token_property_version,<br />    );<br />    // give a new property_version for each <a href="token.md#0x3_token">token</a><br />    <b>while</b> (i &lt; amount) &#123;<br />        <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, token_owner, token_id, keys, values, types);<br />        i &#61; i &#43; 1;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_direct_transfer_script"></a>

## Function `direct_transfer_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_direct_transfer_script">direct_transfer_script</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_direct_transfer_script">direct_transfer_script</a>(<br />    sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    creators_address: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    property_version: u64,<br />    amount: u64,<br />) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creators_address, collection, name, property_version);<br />    <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(sender, receiver, token_id, amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_opt_in_direct_transfer"></a>

## Function `opt_in_direct_transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_opt_in_direct_transfer">opt_in_direct_transfer</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_opt_in_direct_transfer">opt_in_direct_transfer</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br />    <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br />    <b>let</b> opt_in_flag &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(addr).direct_transfer;<br />    &#42;opt_in_flag &#61; opt_in;<br />    <a href="token_event_store.md#0x3_token_event_store_emit_token_opt_in_event">token_event_store::emit_token_opt_in_event</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, opt_in);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfer_with_opt_in"></a>

## Function `transfer_with_opt_in`

Transfers <code>amount</code> of tokens from <code>from</code> to <code><b>to</b></code>.
The receiver <code><b>to</b></code> has to opt&#45;in direct transfer first


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_transfer_with_opt_in">transfer_with_opt_in</a>(from: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_transfer_with_opt_in">transfer_with_opt_in</a>(<br />    from: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    creator: <b>address</b>,<br />    collection_name: String,<br />    token_name: String,<br />    token_property_version: u64,<br />    <b>to</b>: <b>address</b>,<br />    amount: u64,<br />) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator, collection_name, token_name, token_property_version);<br />    <a href="token.md#0x3_token_transfer">transfer</a>(from, token_id, <b>to</b>, amount);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_burn_by_creator"></a>

## Function `burn_by_creator`

Burn a token by creator when the token&apos;s BURNABLE_BY_CREATOR is true
The token is owned at address owner


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn_by_creator">burn_by_creator</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn_by_creator">burn_by_creator</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    owner: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    property_version: u64,<br />    amount: u64,<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT">ENO_BURN_TOKEN_WITH_ZERO_AMOUNT</a>));<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator_address, collection, name, property_version);<br />    <b>let</b> creator_addr &#61; token_id.token_data_id.creator;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>let</b> collections &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br />    <b>assert</b>!(<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;collections.token_data, token_id.token_data_id),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(<br />        &amp;<b>mut</b> collections.token_data,<br />        token_id.token_data_id,<br />    );<br /><br />    // The property should be explicitly set in the <a href="property_map.md#0x3_property_map">property_map</a> for creator <b>to</b> burn the <a href="token.md#0x3_token">token</a><br />    <b>assert</b>!(<br />        <a href="property_map.md#0x3_property_map_contains_key">property_map::contains_key</a>(&amp;token_data.default_properties, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>)),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ECREATOR_CANNOT_BURN_TOKEN">ECREATOR_CANNOT_BURN_TOKEN</a>)<br />    );<br /><br />    <b>let</b> burn_by_creator_flag &#61; <a href="property_map.md#0x3_property_map_read_bool">property_map::read_bool</a>(&amp;token_data.default_properties, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>));<br />    <b>assert</b>!(burn_by_creator_flag, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ECREATOR_CANNOT_BURN_TOKEN">ECREATOR_CANNOT_BURN_TOKEN</a>));<br /><br />    // Burn the tokens.<br />    <b>let</b> <a href="token.md#0x3_token_Token">Token</a> &#123; id: _, amount: burned_amount, token_properties: _ &#125; &#61; <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(owner, token_id, amount);<br />    <b>let</b> token_store &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_BurnToken">BurnToken</a> &#123; id: token_id, amount: burned_amount &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a>&gt;(<br />        &amp;<b>mut</b> token_store.burn_events,<br />        <a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a> &#123; id: token_id, amount: burned_amount &#125;<br />    );<br /><br />    <b>if</b> (token_data.maximum &gt; 0) &#123;<br />        token_data.supply &#61; token_data.supply &#45; burned_amount;<br /><br />        // Delete the token_data <b>if</b> supply drops <b>to</b> 0.<br />        <b>if</b> (token_data.supply &#61;&#61; 0) &#123;<br />            <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> collections.token_data, token_id.token_data_id));<br /><br />            // <b>update</b> the collection supply<br />            <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(<br />                &amp;<b>mut</b> collections.collection_data,<br />                token_id.token_data_id.collection<br />            );<br />            <b>if</b> (collection_data.maximum &gt; 0) &#123;<br />                collection_data.supply &#61; collection_data.supply &#45; 1;<br />                // delete the collection data <b>if</b> the collection supply equals 0<br />                <b>if</b> (collection_data.supply &#61;&#61; 0) &#123;<br />                    <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> collections.collection_data, collection_data.name));<br />                &#125;;<br />            &#125;;<br />        &#125;;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_burn"></a>

## Function `burn`

Burn a token by the token owner


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn">burn</a>(owner: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn">burn</a>(<br />    owner: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    creators_address: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    property_version: u64,<br />    amount: u64<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT">ENO_BURN_TOKEN_WITH_ZERO_AMOUNT</a>));<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creators_address, collection, name, property_version);<br />    <b>let</b> creator_addr &#61; token_id.token_data_id.creator;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>let</b> collections &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br />    <b>assert</b>!(<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;collections.token_data, token_id.token_data_id),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(<br />        &amp;<b>mut</b> collections.token_data,<br />        token_id.token_data_id,<br />    );<br /><br />    <b>assert</b>!(<br />        <a href="property_map.md#0x3_property_map_contains_key">property_map::contains_key</a>(&amp;token_data.default_properties, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>)),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EOWNER_CANNOT_BURN_TOKEN">EOWNER_CANNOT_BURN_TOKEN</a>)<br />    );<br />    <b>let</b> burn_by_owner_flag &#61; <a href="property_map.md#0x3_property_map_read_bool">property_map::read_bool</a>(&amp;token_data.default_properties, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>));<br />    <b>assert</b>!(burn_by_owner_flag, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EOWNER_CANNOT_BURN_TOKEN">EOWNER_CANNOT_BURN_TOKEN</a>));<br /><br />    // Burn the tokens.<br />    <b>let</b> <a href="token.md#0x3_token_Token">Token</a> &#123; id: _, amount: burned_amount, token_properties: _ &#125; &#61; <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(owner, token_id, amount);<br />    <b>let</b> token_store &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner));<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_BurnToken">BurnToken</a> &#123; id: token_id, amount: burned_amount &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a>&gt;(<br />        &amp;<b>mut</b> token_store.burn_events,<br />        <a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a> &#123; id: token_id, amount: burned_amount &#125;<br />    );<br /><br />    // Decrease the supply correspondingly by the amount of tokens burned.<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(<br />        &amp;<b>mut</b> collections.token_data,<br />        token_id.token_data_id,<br />    );<br /><br />    // only <b>update</b> the supply <b>if</b> we tracking the supply and maximal<br />    // maximal &#61;&#61; 0 is reserved for unlimited <a href="token.md#0x3_token">token</a> and collection <b>with</b> no tracking info.<br />    <b>if</b> (token_data.maximum &gt; 0) &#123;<br />        token_data.supply &#61; token_data.supply &#45; burned_amount;<br /><br />        // Delete the token_data <b>if</b> supply drops <b>to</b> 0.<br />        <b>if</b> (token_data.supply &#61;&#61; 0) &#123;<br />            <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> collections.token_data, token_id.token_data_id));<br /><br />            // <b>update</b> the collection supply<br />            <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(<br />                &amp;<b>mut</b> collections.collection_data,<br />                token_id.token_data_id.collection<br />            );<br /><br />            // only <b>update</b> and check the supply for unlimited collection<br />            <b>if</b> (collection_data.maximum &gt; 0)&#123;<br />                collection_data.supply &#61; collection_data.supply &#45; 1;<br />                // delete the collection data <b>if</b> the collection supply equals 0<br />                <b>if</b> (collection_data.supply &#61;&#61; 0) &#123;<br />                    <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> collections.collection_data, collection_data.name));<br />                &#125;;<br />            &#125;;<br />        &#125;;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_collection_description"></a>

## Function `mutate_collection_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_description">mutate_collection_description</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_description">mutate_collection_description</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: String, description: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);<br />    <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data, collection_name);<br />    <b>assert</b>!(collection_data.mutability_config.description, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    <a href="token_event_store.md#0x3_token_event_store_emit_collection_description_mutate_event">token_event_store::emit_collection_description_mutate_event</a>(creator, collection_name, collection_data.description, description);<br />    collection_data.description &#61; description;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_collection_uri"></a>

## Function `mutate_collection_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_uri">mutate_collection_uri</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_uri">mutate_collection_uri</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: String, uri: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);<br />    <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data, collection_name);<br />    <b>assert</b>!(collection_data.mutability_config.uri, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    <a href="token_event_store.md#0x3_token_event_store_emit_collection_uri_mutate_event">token_event_store::emit_collection_uri_mutate_event</a>(creator, collection_name, collection_data.uri , uri);<br />    collection_data.uri &#61; uri;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_collection_maximum"></a>

## Function `mutate_collection_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_maximum">mutate_collection_maximum</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_maximum">mutate_collection_maximum</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: String, maximum: u64) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);<br />    <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data, collection_name);<br />    // cannot change maximum from 0 and cannot change maximum <b>to</b> 0<br />    <b>assert</b>!(collection_data.maximum !&#61; 0 &amp;&amp; maximum !&#61; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));<br />    <b>assert</b>!(maximum &gt;&#61; collection_data.supply, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));<br />    <b>assert</b>!(collection_data.mutability_config.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    <a href="token_event_store.md#0x3_token_event_store_emit_collection_maximum_mutate_event">token_event_store::emit_collection_maximum_mutate_event</a>(creator, collection_name, collection_data.maximum, maximum);<br />    collection_data.maximum &#61; maximum;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_maximum"></a>

## Function `mutate_tokendata_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_maximum">mutate_tokendata_maximum</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, maximum: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_maximum">mutate_tokendata_maximum</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, maximum: u64) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);<br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_data_id);<br />    // cannot change maximum from 0 and cannot change maximum <b>to</b> 0<br />    <b>assert</b>!(token_data.maximum !&#61; 0 &amp;&amp; maximum !&#61; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));<br />    <b>assert</b>!(maximum &gt;&#61; token_data.supply, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));<br />    <b>assert</b>!(token_data.mutability_config.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    <a href="token_event_store.md#0x3_token_event_store_emit_token_maximum_mutate_event">token_event_store::emit_token_maximum_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, token_data.maximum, maximum);<br />    token_data.maximum &#61; maximum;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_uri"></a>

## Function `mutate_tokendata_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_uri">mutate_tokendata_uri</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_uri">mutate_tokendata_uri</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,<br />    uri: String<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);<br /><br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_data_id);<br />    <b>assert</b>!(token_data.mutability_config.uri, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    <a href="token_event_store.md#0x3_token_event_store_emit_token_uri_mutate_event">token_event_store::emit_token_uri_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, token_data.uri ,uri);<br />    token_data.uri &#61; uri;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_royalty"></a>

## Function `mutate_tokendata_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_royalty">mutate_tokendata_royalty</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, royalty: <a href="token.md#0x3_token_Royalty">token::Royalty</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_royalty">mutate_tokendata_royalty</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, royalty: <a href="token.md#0x3_token_Royalty">Royalty</a>) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);<br /><br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_data_id);<br />    <b>assert</b>!(token_data.mutability_config.royalty, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br /><br />    <a href="token_event_store.md#0x3_token_event_store_emit_token_royalty_mutate_event">token_event_store::emit_token_royalty_mutate_event</a>(<br />        creator,<br />        token_data_id.collection,<br />        token_data_id.name,<br />        token_data.royalty.royalty_points_numerator,<br />        token_data.royalty.royalty_points_denominator,<br />        token_data.royalty.payee_address,<br />        royalty.royalty_points_numerator,<br />        royalty.royalty_points_denominator,<br />        royalty.payee_address<br />    );<br />    token_data.royalty &#61; royalty;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_description"></a>

## Function `mutate_tokendata_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_description">mutate_tokendata_description</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_description">mutate_tokendata_description</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, description: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);<br /><br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_data_id);<br />    <b>assert</b>!(token_data.mutability_config.description, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    <a href="token_event_store.md#0x3_token_event_store_emit_token_descrition_mutate_event">token_event_store::emit_token_descrition_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, token_data.description, description);<br />    token_data.description &#61; description;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_property"></a>

## Function `mutate_tokendata_property`

Allow creator to mutate the default properties in TokenData


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_property">mutate_tokendata_property</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_property">mutate_tokendata_property</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);<br />    <b>let</b> key_len &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;keys);<br />    <b>let</b> val_len &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;values);<br />    <b>let</b> typ_len &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;types);<br />    <b>assert</b>!(key_len &#61;&#61; val_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="token.md#0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH">ETOKEN_PROPERTIES_COUNT_NOT_MATCH</a>));<br />    <b>assert</b>!(key_len &#61;&#61; typ_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="token.md#0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH">ETOKEN_PROPERTIES_COUNT_NOT_MATCH</a>));<br /><br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_data_id);<br />    <b>assert</b>!(token_data.mutability_config.properties, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    <b>let</b> i: u64 &#61; 0;<br />    <b>let</b> old_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;PropertyValue&gt;&gt; &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <b>let</b> new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;PropertyValue&gt; &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(&amp;keys);<br />    <b>while</b> (i &lt; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;keys))&#123;<br />        <b>let</b> key &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;keys, i);<br />        <b>let</b> old_pv &#61; <b>if</b> (<a href="property_map.md#0x3_property_map_contains_key">property_map::contains_key</a>(&amp;token_data.default_properties, key)) &#123;<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(&#42;<a href="property_map.md#0x3_property_map_borrow">property_map::borrow</a>(&amp;token_data.default_properties, key))<br />        &#125; <b>else</b> &#123;<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;PropertyValue&gt;()<br />        &#125;;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> old_values, old_pv);<br />        <b>let</b> new_pv &#61; <a href="property_map.md#0x3_property_map_create_property_value_raw">property_map::create_property_value_raw</a>(&#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;values, i), &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;types, i));<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> new_values, new_pv);<br />        <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;old_pv)) &#123;<br />            <a href="property_map.md#0x3_property_map_update_property_value">property_map::update_property_value</a>(&amp;<b>mut</b> token_data.default_properties, key, new_pv);<br />        &#125; <b>else</b> &#123;<br />            <a href="property_map.md#0x3_property_map_add">property_map::add</a>(&amp;<b>mut</b> token_data.default_properties, &#42;key, new_pv);<br />        &#125;;<br />        i &#61; i &#43; 1;<br />    &#125;;<br />    <a href="token_event_store.md#0x3_token_event_store_emit_default_property_mutate_event">token_event_store::emit_default_property_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, keys, old_values, new_values);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mutate_one_token"></a>

## Function `mutate_one_token`

Mutate the token_properties of one token.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_owner: <b>address</b>,<br />    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />): <a href="token.md#0x3_token_TokenId">TokenId</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> creator &#61; token_id.token_data_id.creator;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>) &#61;&#61; creator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>));<br />    // validate <b>if</b> the properties is mutable<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(<br />        creator<br />    ).token_data;<br /><br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_id.token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_id.token_data_id);<br /><br />    // <b>if</b> default property is mutatable, <a href="token.md#0x3_token">token</a> property is alwasy mutable<br />    // we only need <b>to</b> check <a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a> when default property is immutable<br />    <b>if</b> (!token_data.mutability_config.properties) &#123;<br />        <b>assert</b>!(<br />            <a href="property_map.md#0x3_property_map_contains_key">property_map::contains_key</a>(&amp;token_data.default_properties, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>)),<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>)<br />        );<br /><br />        <b>let</b> token_prop_mutable &#61; <a href="property_map.md#0x3_property_map_read_bool">property_map::read_bool</a>(&amp;token_data.default_properties, &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>));<br />        <b>assert</b>!(token_prop_mutable, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));<br />    &#125;;<br /><br />    // check <b>if</b> the property_version is 0 <b>to</b> determine <b>if</b> we need <b>to</b> <b>update</b> the property_version<br />    <b>if</b> (token_id.property_version &#61;&#61; 0) &#123;<br />        <b>let</b> <a href="token.md#0x3_token">token</a> &#61; <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(token_owner, token_id, 1);<br />        // give a new property_version for each <a href="token.md#0x3_token">token</a><br />        <b>let</b> cur_property_version &#61; token_data.largest_property_version &#43; 1;<br />        <b>let</b> new_token_id &#61; <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_id.token_data_id, cur_property_version);<br />        <b>let</b> new_token &#61; <a href="token.md#0x3_token_Token">Token</a> &#123;<br />            id: new_token_id,<br />            amount: 1,<br />            token_properties: token_data.default_properties,<br />        &#125;;<br />        <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(token_owner, new_token);<br />        <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner, new_token_id, keys, values, types);<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_MutateTokenPropertyMap">MutateTokenPropertyMap</a> &#123;<br />                old_id: token_id,<br />                new_id: new_token_id,<br />                keys,<br />                values,<br />                types<br />            &#125;);<br />        &#125;;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a>&gt;(<br />            &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(token_owner).mutate_token_property_events,<br />            <a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a> &#123;<br />                old_id: token_id,<br />                new_id: new_token_id,<br />                keys,<br />                values,<br />                types<br />            &#125;,<br />        );<br /><br />        token_data.largest_property_version &#61; cur_property_version;<br />        // burn the orignial property_version 0 <a href="token.md#0x3_token">token</a> after mutation<br />        <b>let</b> <a href="token.md#0x3_token_Token">Token</a> &#123; id: _, amount: _, token_properties: _ &#125; &#61; <a href="token.md#0x3_token">token</a>;<br />        new_token_id<br />    &#125; <b>else</b> &#123;<br />        // only 1 <b>copy</b> for the <a href="token.md#0x3_token">token</a> <b>with</b> property verion bigger than 0<br />        <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner, token_id, keys, values, types);<br />        <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_MutateTokenPropertyMap">MutateTokenPropertyMap</a> &#123;<br />                old_id: token_id,<br />                new_id: token_id,<br />                keys,<br />                values,<br />                types<br />            &#125;);<br />        &#125;;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a>&gt;(<br />            &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(token_owner).mutate_token_property_events,<br />            <a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a> &#123;<br />                old_id: token_id,<br />                new_id: token_id,<br />                keys,<br />                values,<br />                types<br />            &#125;,<br />        );<br />        token_id<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_royalty"></a>

## Function `create_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: <b>address</b>): <a href="token.md#0x3_token_Royalty">token::Royalty</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: <b>address</b>): <a href="token.md#0x3_token_Royalty">Royalty</a> &#123;<br />    <b>assert</b>!(royalty_points_numerator &lt;&#61; royalty_points_denominator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR">EINVALID_ROYALTY_NUMERATOR_DENOMINATOR</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/doc/account.md#0x1_account_exists_at">account::exists_at</a>(payee_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST">EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST</a>));<br />    <a href="token.md#0x3_token_Royalty">Royalty</a> &#123;<br />        royalty_points_numerator,<br />        royalty_points_denominator,<br />        payee_address<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_deposit_token"></a>

## Function `deposit_token`

Deposit the token balance into the owner&apos;s account and emit an event.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">Token</a>) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br />    <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br />    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr, <a href="token.md#0x3_token">token</a>)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_direct_deposit_with_opt_in"></a>

## Function `direct_deposit_with_opt_in`

direct deposit if user opt in direct transfer


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_deposit_with_opt_in">direct_deposit_with_opt_in</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_deposit_with_opt_in">direct_deposit_with_opt_in</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">Token</a>) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> opt_in_transfer &#61; <b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr).direct_transfer;<br />    <b>assert</b>!(opt_in_transfer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>));<br />    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr, <a href="token.md#0x3_token">token</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_direct_transfer"></a>

## Function `direct_transfer`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(<br />    sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,<br />    amount: u64,<br />) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> <a href="token.md#0x3_token">token</a> &#61; <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(sender, token_id, amount);<br />    <a href="token.md#0x3_token_deposit_token">deposit_token</a>(receiver, <a href="token.md#0x3_token">token</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_initialize_token_store"></a>

## Function `initialize_token_store`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>))) &#123;<br />        <b>move_to</b>(<br />            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,<br />            <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />                tokens: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),<br />                direct_transfer: <b>false</b>,<br />                deposit_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_DepositEvent">DepositEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />                withdraw_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />                burn_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />                mutate_token_property_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),<br />            &#125;,<br />        );<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_merge"></a>

## Function `merge`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_merge">merge</a>(dst_token: &amp;<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, source_token: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_merge">merge</a>(dst_token: &amp;<b>mut</b> <a href="token.md#0x3_token_Token">Token</a>, source_token: <a href="token.md#0x3_token_Token">Token</a>) &#123;<br />    <b>assert</b>!(&amp;dst_token.id &#61;&#61; &amp;source_token.id, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_TOKEN_MERGE">EINVALID_TOKEN_MERGE</a>));<br />    dst_token.amount &#61; dst_token.amount &#43; source_token.amount;<br />    <b>let</b> <a href="token.md#0x3_token_Token">Token</a> &#123; id: _, amount: _, token_properties: _ &#125; &#61; source_token;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_split"></a>

## Function `split`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_split">split</a>(dst_token: &amp;<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_split">split</a>(dst_token: &amp;<b>mut</b> <a href="token.md#0x3_token_Token">Token</a>, amount: u64): <a href="token.md#0x3_token_Token">Token</a> &#123;<br />    <b>assert</b>!(dst_token.id.property_version &#61;&#61; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="token.md#0x3_token_ENFT_NOT_SPLITABLE">ENFT_NOT_SPLITABLE</a>));<br />    <b>assert</b>!(dst_token.amount &gt; amount, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT">ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT</a>));<br />    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT">ETOKEN_CANNOT_HAVE_ZERO_AMOUNT</a>));<br />    dst_token.amount &#61; dst_token.amount &#45; amount;<br />    <a href="token.md#0x3_token_Token">Token</a> &#123;<br />        id: dst_token.id,<br />        amount,<br />        token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>(),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_token_id"></a>

## Function `token_id`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_token_id">token_id</a>(<a href="token.md#0x3_token">token</a>: &amp;<a href="token.md#0x3_token_Token">token::Token</a>): &amp;<a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_token_id">token_id</a>(<a href="token.md#0x3_token">token</a>: &amp;<a href="token.md#0x3_token_Token">Token</a>): &amp;<a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />    &amp;<a href="token.md#0x3_token">token</a>.id<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of tokens from <code>from</code> to <code><b>to</b></code>.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_transfer">transfer</a>(from: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_transfer">transfer</a>(<br />    from: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    id: <a href="token.md#0x3_token_TokenId">TokenId</a>,<br />    <b>to</b>: <b>address</b>,<br />    amount: u64,<br />) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> opt_in_transfer &#61; <b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(<b>to</b>).direct_transfer;<br />    <b>assert</b>!(opt_in_transfer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>));<br />    <b>let</b> <a href="token.md#0x3_token">token</a> &#61; <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(from, id, amount);<br />    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(<b>to</b>, <a href="token.md#0x3_token">token</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_withdraw_capability"></a>

## Function `create_withdraw_capability`

Token owner can create this one&#45;time withdraw capability with an expiration time


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_withdraw_capability">create_withdraw_capability</a>(owner: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64, expiration_sec: u64): <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_withdraw_capability">create_withdraw_capability</a>(<br />    owner: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,<br />    amount: u64,<br />    expiration_sec: u64,<br />): <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> &#123;<br />    <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> &#123;<br />        token_owner: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner),<br />        token_id,<br />        amount,<br />        expiration_sec,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_withdraw_with_capability"></a>

## Function `withdraw_with_capability`

Withdraw the token with a capability


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_with_capability">withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_with_capability">withdraw_with_capability</a>(<br />    withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>,<br />): <a href="token.md#0x3_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    // verify the delegation hasn&apos;t expired yet<br />    <b>assert</b>!(<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;&#61; withdraw_proof.expiration_sec, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EWITHDRAW_PROOF_EXPIRES">EWITHDRAW_PROOF_EXPIRES</a>));<br /><br />    <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(<br />        withdraw_proof.token_owner,<br />        withdraw_proof.token_id,<br />        withdraw_proof.amount,<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_partial_withdraw_with_capability"></a>

## Function `partial_withdraw_with_capability`

Withdraw the token with a capability.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_partial_withdraw_with_capability">partial_withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>, withdraw_amount: u64): (<a href="token.md#0x3_token_Token">token::Token</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_partial_withdraw_with_capability">partial_withdraw_with_capability</a>(<br />    withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>,<br />    withdraw_amount: u64,<br />): (<a href="token.md#0x3_token_Token">Token</a>, Option&lt;<a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>&gt;) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    // verify the delegation hasn&apos;t expired yet<br />    <b>assert</b>!(<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;&#61; withdraw_proof.expiration_sec, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EWITHDRAW_PROOF_EXPIRES">EWITHDRAW_PROOF_EXPIRES</a>));<br /><br />    <b>assert</b>!(withdraw_amount &lt;&#61; withdraw_proof.amount, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT">EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT</a>));<br /><br />    <b>let</b> res: Option&lt;<a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>&gt; &#61; <b>if</b> (withdraw_amount &#61;&#61; withdraw_proof.amount) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;<a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>&gt;()<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<br />            <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> &#123;<br />                token_owner: withdraw_proof.token_owner,<br />                token_id: withdraw_proof.token_id,<br />                amount: withdraw_proof.amount &#45; withdraw_amount,<br />                expiration_sec: withdraw_proof.expiration_sec,<br />            &#125;<br />        )<br />    &#125;;<br /><br />    (<br />        <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(<br />            withdraw_proof.token_owner,<br />            withdraw_proof.token_id,<br />            withdraw_amount,<br />        ),<br />        res<br />    )<br /><br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_withdraw_token"></a>

## Function `withdraw_token`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    id: <a href="token.md#0x3_token_TokenId">TokenId</a>,<br />    amount: u64,<br />): <a href="token.md#0x3_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br />    <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(account_addr, id, amount)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_collection"></a>

## Function `create_collection`

Create a new collection to hold tokens


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection">create_collection</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection">create_collection</a>(<br />    creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    name: String,<br />    description: String,<br />    uri: String,<br />    maximum: u64,<br />    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr)) &#123;<br />        <b>move_to</b>(<br />            creator,<br />            <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />                collection_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),<br />                token_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),<br />                create_collection_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a>&gt;(creator),<br />                create_token_data_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a>&gt;(creator),<br />                mint_token_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a>&gt;(creator),<br />            &#125;,<br />        )<br />    &#125;;<br /><br />    <b>let</b> collection_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr).collection_data;<br /><br />    <b>assert</b>!(<br />        !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(collection_data, name),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="token.md#0x3_token_ECOLLECTION_ALREADY_EXISTS">ECOLLECTION_ALREADY_EXISTS</a>),<br />    );<br /><br />    <b>let</b> mutability_config &#61; <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(&amp;mutate_setting);<br />    <b>let</b> collection &#61; <a href="token.md#0x3_token_CollectionData">CollectionData</a> &#123;<br />        description,<br />        name: name,<br />        uri,<br />        supply: 0,<br />        maximum,<br />        mutability_config<br />    &#125;;<br /><br />    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(collection_data, name, collection);<br />    <b>let</b> collection_handle &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token.md#0x3_token_CreateCollection">CreateCollection</a> &#123;<br />                creator: account_addr,<br />                collection_name: name,<br />                uri,<br />                description,<br />                maximum,<br />            &#125;<br />        );<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a>&gt;(<br />        &amp;<b>mut</b> collection_handle.create_collection_events,<br />        <a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a> &#123;<br />            creator: account_addr,<br />            collection_name: name,<br />            uri,<br />            description,<br />            maximum,<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_collection_exists">check_collection_exists</a>(creator: <b>address</b>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_collection_exists">check_collection_exists</a>(creator: <b>address</b>, name: String): bool <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>let</b> collection_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).collection_data;<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(collection_data, name)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_check_tokendata_exists"></a>

## Function `check_tokendata_exists`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_tokendata_exists">check_tokendata_exists</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_tokendata_exists">check_tokendata_exists</a>(creator: <b>address</b>, collection_name: String, token_name: String): bool <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>let</b> token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).token_data;<br />    <b>let</b> token_data_id &#61; <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator, collection_name, token_name);<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(token_data, token_data_id)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_tokendata"></a>

## Function `create_tokendata`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    collection: String,<br />    name: String,<br />    description: String,<br />    maximum: u64,<br />    uri: String,<br />    royalty_payee_address: <b>address</b>,<br />    royalty_points_denominator: u64,<br />    royalty_points_numerator: u64,<br />    token_mutate_config: <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>,<br />    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;<br />): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENFT_NAME_TOO_LONG">ENFT_NAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;collection) &lt;&#61; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;uri) &lt;&#61; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));<br />    <b>assert</b>!(royalty_points_numerator &lt;&#61; royalty_points_denominator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR">EINVALID_ROYALTY_NUMERATOR_DENOMINATOR</a>));<br /><br />    <b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),<br />    );<br />    <b>let</b> collections &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr);<br /><br />    <b>let</b> token_data_id &#61; <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(account_addr, collection, name);<br /><br />    <b>assert</b>!(<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;collections.collection_data, token_data_id.collection),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>),<br />    );<br />    <b>assert</b>!(<br />        !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;collections.token_data, token_data_id),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="token.md#0x3_token_ETOKEN_DATA_ALREADY_EXISTS">ETOKEN_DATA_ALREADY_EXISTS</a>),<br />    );<br /><br />    <b>let</b> collection &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> collections.collection_data, token_data_id.collection);<br /><br />    // <b>if</b> collection maximum &#61;&#61; 0, user don&apos;t want <b>to</b> enforce supply constraint.<br />    // we don&apos;t track supply <b>to</b> make <a href="token.md#0x3_token">token</a> creation parallelizable<br />    <b>if</b> (collection.maximum &gt; 0) &#123;<br />        collection.supply &#61; collection.supply &#43; 1;<br />        <b>assert</b>!(<br />            collection.maximum &gt;&#61; collection.supply,<br />            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM">ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM</a>),<br />        );<br />    &#125;;<br /><br />    <b>let</b> token_data &#61; <a href="token.md#0x3_token_TokenData">TokenData</a> &#123;<br />        maximum,<br />        largest_property_version: 0,<br />        supply: 0,<br />        uri,<br />        royalty: <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator, royalty_points_denominator, royalty_payee_address),<br />        name,<br />        description,<br />        default_properties: <a href="property_map.md#0x3_property_map_new">property_map::new</a>(property_keys, property_values, property_types),<br />        mutability_config: token_mutate_config,<br />    &#125;;<br /><br />    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> collections.token_data, token_data_id, token_data);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<br />            <a href="token.md#0x3_token_CreateTokenData">CreateTokenData</a> &#123;<br />                id: token_data_id,<br />                description,<br />                maximum,<br />                uri,<br />                royalty_payee_address,<br />                royalty_points_denominator,<br />                royalty_points_numerator,<br />                name,<br />                mutability_config: token_mutate_config,<br />                property_keys,<br />                property_values,<br />                property_types,<br />            &#125;<br />        );<br />    &#125;;<br /><br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a>&gt;(<br />        &amp;<b>mut</b> collections.create_token_data_events,<br />        <a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a> &#123;<br />            id: token_data_id,<br />            description,<br />            maximum,<br />            uri,<br />            royalty_payee_address,<br />            royalty_points_denominator,<br />            royalty_points_numerator,<br />            name,<br />            mutability_config: token_mutate_config,<br />            property_keys,<br />            property_values,<br />            property_types,<br />        &#125;,<br />    );<br />    token_data_id<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_supply"></a>

## Function `get_collection_supply`

return the number of distinct token_data_id created under this collection


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_supply">get_collection_supply</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_supply">get_collection_supply</a>(creator_address: <b>address</b>, collection_name: String): Option&lt;u64&gt; <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);<br />    <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data, collection_name);<br /><br />    <b>if</b> (collection_data.maximum &gt; 0) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(collection_data.supply)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_description"></a>

## Function `get_collection_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_description">get_collection_description</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_description">get_collection_description</a>(creator_address: <b>address</b>, collection_name: String): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);<br />    <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data, collection_name);<br />    collection_data.description<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_uri"></a>

## Function `get_collection_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_uri">get_collection_uri</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_uri">get_collection_uri</a>(creator_address: <b>address</b>, collection_name: String): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);<br />    <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data, collection_name);<br />    collection_data.uri<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_maximum"></a>

## Function `get_collection_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_maximum">get_collection_maximum</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_maximum">get_collection_maximum</a>(creator_address: <b>address</b>, collection_name: String): u64 <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);<br />    <b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data, collection_name);<br />    collection_data.maximum<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_supply"></a>

## Function `get_token_supply`

return the number of distinct token_id created under this TokenData


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_supply">get_token_supply</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_supply">get_token_supply</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): Option&lt;u64&gt; <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_data_id);<br /><br />    <b>if</b> (token_data.maximum &gt; 0) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(token_data.supply)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u64&gt;()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_tokendata_largest_property_version"></a>

## Function `get_tokendata_largest_property_version`

return the largest_property_version of this TokenData


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_largest_property_version">get_tokendata_largest_property_version</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_largest_property_version">get_tokendata_largest_property_version</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): u64 <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_data_id).largest_property_version<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_id"></a>

## Function `get_token_id`

return the TokenId for a given Token


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id">get_token_id</a>(<a href="token.md#0x3_token">token</a>: &amp;<a href="token.md#0x3_token_Token">token::Token</a>): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id">get_token_id</a>(<a href="token.md#0x3_token">token</a>: &amp;<a href="token.md#0x3_token_Token">Token</a>): <a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />    <a href="token.md#0x3_token">token</a>.id<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_direct_transfer"></a>

## Function `get_direct_transfer`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_direct_transfer">get_direct_transfer</a>(receiver: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_direct_transfer">get_direct_transfer</a>(receiver: <b>address</b>): bool <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver)) &#123;<br />        <b>return</b> <b>false</b><br />    &#125;;<br /><br />    <b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver).direct_transfer<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_token_mutability_config"></a>

## Function `create_token_mutability_config`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(mutate_setting: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(mutate_setting: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> &#123;<br />    <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> &#123;<br />        maximum: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_TOKEN_MAX_MUTABLE_IND">TOKEN_MAX_MUTABLE_IND</a>),<br />        uri: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_TOKEN_URI_MUTABLE_IND">TOKEN_URI_MUTABLE_IND</a>),<br />        royalty: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_TOKEN_ROYALTY_MUTABLE_IND">TOKEN_ROYALTY_MUTABLE_IND</a>),<br />        description: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND">TOKEN_DESCRIPTION_MUTABLE_IND</a>),<br />        properties: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE_IND">TOKEN_PROPERTY_MUTABLE_IND</a>),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_collection_mutability_config"></a>

## Function `create_collection_mutability_config`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(mutate_setting: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(mutate_setting: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> &#123;<br />    <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> &#123;<br />        description: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND">COLLECTION_DESCRIPTION_MUTABLE_IND</a>),<br />        uri: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_COLLECTION_URI_MUTABLE_IND">COLLECTION_URI_MUTABLE_IND</a>),<br />        maximum: &#42;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mutate_setting, <a href="token.md#0x3_token_COLLECTION_MAX_MUTABLE_IND">COLLECTION_MAX_MUTABLE_IND</a>),<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mint_token"></a>

## Function `mint_token`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token">mint_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token">mint_token</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,<br />    amount: u64,<br />): <a href="token.md#0x3_token_TokenId">TokenId</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>assert</b>!(token_data_id.creator &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>));<br />    <b>let</b> creator_addr &#61; token_data_id.creator;<br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_data_id);<br /><br />    <b>if</b> (token_data.maximum &gt; 0) &#123;<br />        <b>assert</b>!(token_data.supply &#43; amount &lt;&#61; token_data.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM">EMINT_WOULD_EXCEED_TOKEN_MAXIMUM</a>));<br />        token_data.supply &#61; token_data.supply &#43; amount;<br />    &#125;;<br /><br />    // we add more tokens <b>with</b> property_version 0<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_MintToken">MintToken</a> &#123; id: token_data_id, amount &#125;)<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a>&gt;(<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).mint_token_events,<br />        <a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a> &#123;<br />            id: token_data_id,<br />            amount,<br />        &#125;<br />    );<br /><br />    <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,<br />        <a href="token.md#0x3_token_Token">Token</a> &#123;<br />            id: token_id,<br />            amount,<br />            token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>(), // same <b>as</b> default properties no need <b>to</b> store<br />        &#125;<br />    );<br /><br />    token_id<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_mint_token_to"></a>

## Function `mint_token_to`

create tokens and directly deposite to receiver&apos;s address. The receiver should opt&#45;in direct transfer


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token_to">mint_token_to</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token_to">mint_token_to</a>(<br />    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    receiver: <b>address</b>,<br />    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,<br />    amount: u64,<br />) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>));<br />    <b>let</b> opt_in_transfer &#61; <b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver).direct_transfer;<br />    <b>assert</b>!(opt_in_transfer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>));<br /><br />    <b>assert</b>!(token_data_id.creator &#61;&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>));<br />    <b>let</b> creator_addr &#61; token_data_id.creator;<br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(all_token_data, token_data_id);<br /><br />    <b>if</b> (token_data.maximum &gt; 0) &#123;<br />        <b>assert</b>!(token_data.supply &#43; amount &lt;&#61; token_data.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM">EMINT_WOULD_EXCEED_TOKEN_MAXIMUM</a>));<br />        token_data.supply &#61; token_data.supply &#43; amount;<br />    &#125;;<br /><br />    // we add more tokens <b>with</b> property_version 0<br />    <b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_MintToken">MintToken</a> &#123; id: token_data_id, amount &#125;)<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a>&gt;(<br />        &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).mint_token_events,<br />        <a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a> &#123;<br />            id: token_data_id,<br />            amount,<br />        &#125;<br />    );<br /><br />    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(receiver,<br />        <a href="token.md#0x3_token_Token">Token</a> &#123;<br />            id: token_id,<br />            amount,<br />            token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>(), // same <b>as</b> default properties no need <b>to</b> store<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_token_id"></a>

## Function `create_token_id`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />    <a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />        token_data_id,<br />        property_version,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_token_data_id"></a>

## Function `create_token_data_id`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(<br />    creator: <b>address</b>,<br />    collection: String,<br />    name: String,<br />): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;collection) &lt;&#61; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(&amp;name) &lt;&#61; <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENFT_NAME_TOO_LONG">ENFT_NAME_TOO_LONG</a>));<br />    <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123; creator, collection, name &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_create_token_id_raw"></a>

## Function `create_token_id_raw`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(<br />    creator: <b>address</b>,<br />    collection: String,<br />    name: String,<br />    property_version: u64,<br />): <a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />    <a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />        token_data_id: <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator, collection, name),<br />        property_version,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_balance_of"></a>

## Function `balance_of`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_balance_of">balance_of</a>(owner: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_balance_of">balance_of</a>(owner: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">TokenId</a>): u64 <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner)) &#123;<br />        <b>return</b> 0<br />    &#125;;<br />    <b>let</b> token_store &#61; <b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner);<br />    <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;token_store.tokens, id)) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&amp;token_store.tokens, id).amount<br />    &#125; <b>else</b> &#123;<br />        0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_has_token_store"></a>

## Function `has_token_store`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_has_token_store">has_token_store</a>(owner: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_has_token_store">has_token_store</a>(owner: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_royalty"></a>

## Function `get_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty">get_royalty</a>(token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty">get_royalty</a>(token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>): <a href="token.md#0x3_token_Royalty">Royalty</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> token_data_id &#61; token_id.token_data_id;<br />    <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id)<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_royalty_numerator"></a>

## Function `get_royalty_numerator`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_numerator">get_royalty_numerator</a>(royalty: &amp;<a href="token.md#0x3_token_Royalty">token::Royalty</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_numerator">get_royalty_numerator</a>(royalty: &amp;<a href="token.md#0x3_token_Royalty">Royalty</a>): u64 &#123;<br />    royalty.royalty_points_numerator<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_royalty_denominator"></a>

## Function `get_royalty_denominator`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_denominator">get_royalty_denominator</a>(royalty: &amp;<a href="token.md#0x3_token_Royalty">token::Royalty</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_denominator">get_royalty_denominator</a>(royalty: &amp;<a href="token.md#0x3_token_Royalty">Royalty</a>): u64 &#123;<br />    royalty.royalty_points_denominator<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_royalty_payee"></a>

## Function `get_royalty_payee`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_payee">get_royalty_payee</a>(royalty: &amp;<a href="token.md#0x3_token_Royalty">token::Royalty</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_payee">get_royalty_payee</a>(royalty: &amp;<a href="token.md#0x3_token_Royalty">Royalty</a>): <b>address</b> &#123;<br />    royalty.payee_address<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_amount"></a>

## Function `get_token_amount`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_amount">get_token_amount</a>(<a href="token.md#0x3_token">token</a>: &amp;<a href="token.md#0x3_token_Token">token::Token</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_amount">get_token_amount</a>(<a href="token.md#0x3_token">token</a>: &amp;<a href="token.md#0x3_token_Token">Token</a>): u64 &#123;<br />    <a href="token.md#0x3_token">token</a>.amount<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_id_fields"></a>

## Function `get_token_id_fields`

return the creator address, collection name, token name and property_version


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id_fields">get_token_id_fields</a>(token_id: &amp;<a href="token.md#0x3_token_TokenId">token::TokenId</a>): (<b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id_fields">get_token_id_fields</a>(token_id: &amp;<a href="token.md#0x3_token_TokenId">TokenId</a>): (<b>address</b>, String, String, u64) &#123;<br />    (<br />        token_id.token_data_id.creator,<br />        token_id.token_data_id.collection,<br />        token_id.token_data_id.name,<br />        token_id.property_version,<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_data_id_fields"></a>

## Function `get_token_data_id_fields`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_data_id_fields">get_token_data_id_fields</a>(token_data_id: &amp;<a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): (<b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_data_id_fields">get_token_data_id_fields</a>(token_data_id: &amp;<a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): (<b>address</b>, String, String) &#123;<br />    (<br />        token_data_id.creator,<br />        token_data_id.collection,<br />        token_data_id.name,<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_property_map"></a>

## Function `get_property_map`

return a copy of the token property map.
if property_version &#61; 0, return the default property map
if property_version &gt; 0, return the property value stored at owner&apos;s token store


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_property_map">get_property_map</a>(owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_property_map">get_property_map</a>(owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>): PropertyMap <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>assert</b>!(<a href="token.md#0x3_token_balance_of">balance_of</a>(owner, token_id) &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));<br />    // <b>if</b> property_version &#61; 0, <b>return</b> default property map<br />    <b>if</b> (token_id.property_version &#61;&#61; 0) &#123;<br />        <b>let</b> creator_addr &#61; token_id.token_data_id.creator;<br />        <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br />        <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_id.token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />        <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_id.token_data_id);<br />        token_data.default_properties<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> tokens &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner).tokens;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(tokens, token_id).token_properties<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_tokendata_maximum"></a>

## Function `get_tokendata_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_maximum">get_tokendata_maximum</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_maximum">get_tokendata_maximum</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): u64 <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> creator_address &#61; token_data_id.creator;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br /><br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_data_id);<br />    token_data.maximum<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_tokendata_uri"></a>

## Function `get_tokendata_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_uri">get_tokendata_uri</a>(creator: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_uri">get_tokendata_uri</a>(creator: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br /><br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_data_id);<br />    token_data.uri<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_tokendata_description"></a>

## Function `get_tokendata_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_description">get_tokendata_description</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_description">get_tokendata_description</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> creator_address &#61; token_data_id.creator;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br /><br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_data_id);<br />    token_data.description<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_tokendata_royalty"></a>

## Function `get_tokendata_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): <a href="token.md#0x3_token_Royalty">Royalty</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> creator_address &#61; token_data_id.creator;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br /><br />    <b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_data_id);<br />    token_data.royalty<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_tokendata_id"></a>

## Function `get_tokendata_id`

return the token_data_id from the token_id


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_id">get_tokendata_id</a>(token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_id">get_tokendata_id</a>(token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123;<br />    token_id.token_data_id<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_tokendata_mutability_config"></a>

## Function `get_tokendata_mutability_config`

return the mutation setting of the token


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_mutability_config">get_tokendata_mutability_config</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_mutability_config">get_tokendata_mutability_config</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> creator_addr &#61; token_data_id.creator;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_token_data, token_data_id).mutability_config<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_mutability_maximum"></a>

## Function `get_token_mutability_maximum`

return if the token&apos;s maximum is mutable


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_maximum">get_token_mutability_maximum</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_maximum">get_token_mutability_maximum</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool &#123;<br />    config.maximum<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_mutability_royalty"></a>

## Function `get_token_mutability_royalty`

return if the token royalty is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_royalty">get_token_mutability_royalty</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_royalty">get_token_mutability_royalty</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool &#123;<br />    config.royalty<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_mutability_uri"></a>

## Function `get_token_mutability_uri`

return if the token uri is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_uri">get_token_mutability_uri</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_uri">get_token_mutability_uri</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool &#123;<br />    config.uri<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_mutability_description"></a>

## Function `get_token_mutability_description`

return if the token description is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_description">get_token_mutability_description</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_description">get_token_mutability_description</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool &#123;<br />    config.description<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_token_mutability_default_properties"></a>

## Function `get_token_mutability_default_properties`

return if the tokendata&apos;s default properties is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_default_properties">get_token_mutability_default_properties</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_default_properties">get_token_mutability_default_properties</a>(config: &amp;<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool &#123;<br />    config.properties<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_config"></a>

## Function `get_collection_mutability_config`

return the collection mutation setting


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_config">get_collection_mutability_config</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_config">get_collection_mutability_config</a>(<br />    creator: <b>address</b>,<br />    collection_name: String<br />): <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_collection_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).collection_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_collection_data, collection_name), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>));<br />    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(all_collection_data, collection_name).mutability_config<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_description"></a>

## Function `get_collection_mutability_description`

return if the collection description is mutable with a collection mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_description">get_collection_mutability_description</a>(config: &amp;<a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_description">get_collection_mutability_description</a>(config: &amp;<a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a>): bool &#123;<br />    config.description<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_uri"></a>

## Function `get_collection_mutability_uri`

return if the collection uri is mutable with a collection mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_uri">get_collection_mutability_uri</a>(config: &amp;<a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_uri">get_collection_mutability_uri</a>(config: &amp;<a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a>): bool &#123;<br />    config.uri<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_maximum"></a>

## Function `get_collection_mutability_maximum`

return if the collection maximum is mutable with collection mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_maximum">get_collection_mutability_maximum</a>(config: &amp;<a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_maximum">get_collection_mutability_maximum</a>(config: &amp;<a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a>): bool &#123;<br />    config.maximum<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_destroy_token_data"></a>

## Function `destroy_token_data`



<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(token_data: <a href="token.md#0x3_token_TokenData">token::TokenData</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(token_data: <a href="token.md#0x3_token_TokenData">TokenData</a>) &#123;<br />    <b>let</b> <a href="token.md#0x3_token_TokenData">TokenData</a> &#123;<br />        maximum: _,<br />        largest_property_version: _,<br />        supply: _,<br />        uri: _,<br />        royalty: _,<br />        name: _,<br />        description: _,<br />        default_properties: _,<br />        mutability_config: _,<br />    &#125; &#61; token_data;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_destroy_collection_data"></a>

## Function `destroy_collection_data`



<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(collection_data: <a href="token.md#0x3_token_CollectionData">token::CollectionData</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(collection_data: <a href="token.md#0x3_token_CollectionData">CollectionData</a>) &#123;<br />    <b>let</b> <a href="token.md#0x3_token_CollectionData">CollectionData</a> &#123;<br />        description: _,<br />        name: _,<br />        uri: _,<br />        supply: _,<br />        maximum: _,<br />        mutability_config: _,<br />    &#125; &#61; collection_data;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_withdraw_with_event_internal"></a>

## Function `withdraw_with_event_internal`



<pre><code><b>fun</b> <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(account_addr: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(<br />    account_addr: <b>address</b>,<br />    id: <a href="token.md#0x3_token_TokenId">TokenId</a>,<br />    amount: u64,<br />): <a href="token.md#0x3_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    // It does not make sense <b>to</b> withdraw 0 tokens.<br />    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EWITHDRAW_ZERO">EWITHDRAW_ZERO</a>));<br />    // Make sure the <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> <b>has</b> sufficient tokens <b>to</b> withdraw.<br />    <b>assert</b>!(<a href="token.md#0x3_token_balance_of">balance_of</a>(account_addr, id) &gt;&#61; amount, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));<br /><br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>let</b> token_store &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);<br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_Withdraw">Withdraw</a> &#123; id, amount &#125;)<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a>&gt;(<br />        &amp;<b>mut</b> token_store.withdraw_events,<br />        <a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a> &#123; id, amount &#125;<br />    );<br />    <b>let</b> tokens &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr).tokens;<br />    <b>assert</b>!(<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(tokens, id),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ENO_TOKEN_IN_TOKEN_STORE">ENO_TOKEN_IN_TOKEN_STORE</a>),<br />    );<br />    // balance &gt; amount and amount &gt; 0 indirectly asserted that balance &gt; 0.<br />    <b>let</b> balance &#61; &amp;<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(tokens, id).amount;<br />    <b>if</b> (&#42;balance &gt; amount) &#123;<br />        &#42;balance &#61; &#42;balance &#45; amount;<br />        <a href="token.md#0x3_token_Token">Token</a> &#123; id, amount, token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>() &#125;<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(tokens, id)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_update_token_property_internal"></a>

## Function `update_token_property_internal`



<pre><code><b>fun</b> <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(<br />    token_owner: <b>address</b>,<br />    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,<br />    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>let</b> tokens &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(token_owner).tokens;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(tokens, token_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ENO_TOKEN_IN_TOKEN_STORE">ENO_TOKEN_IN_TOKEN_STORE</a>));<br /><br />    <b>let</b> value &#61; &amp;<b>mut</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(tokens, token_id).token_properties;<br />    <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(&amp;keys);<br />    <a href="property_map.md#0x3_property_map_update_property_map">property_map::update_property_map</a>(value, keys, values, types);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_direct_deposit"></a>

## Function `direct_deposit`

Deposit the token balance into the recipients account and emit an event.


<pre><code><b>fun</b> <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">Token</a>) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> &#123;<br />    <b>assert</b>!(<a href="token.md#0x3_token">token</a>.amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT">ETOKEN_CANNOT_HAVE_ZERO_AMOUNT</a>));<br />    <b>let</b> token_store &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_Deposit">Deposit</a> &#123; id: <a href="token.md#0x3_token">token</a>.id, amount: <a href="token.md#0x3_token">token</a>.amount &#125;);<br />    &#125;;<br />    <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_DepositEvent">DepositEvent</a>&gt;(<br />        &amp;<b>mut</b> token_store.deposit_events,<br />        <a href="token.md#0x3_token_DepositEvent">DepositEvent</a> &#123; id: <a href="token.md#0x3_token">token</a>.id, amount: <a href="token.md#0x3_token">token</a>.amount &#125;,<br />    );<br /><br />    <b>assert</b>!(<br />        <b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr),<br />        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>),<br />    );<br /><br />    <b>if</b> (!<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&amp;token_store.tokens, <a href="token.md#0x3_token">token</a>.id)) &#123;<br />        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> token_store.tokens, <a href="token.md#0x3_token">token</a>.id, <a href="token.md#0x3_token">token</a>);<br />    &#125; <b>else</b> &#123;<br />        <b>let</b> recipient_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> token_store.tokens, <a href="token.md#0x3_token">token</a>.id);<br />        <a href="token.md#0x3_token_merge">merge</a>(recipient_token, <a href="token.md#0x3_token">token</a>);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_assert_collection_exists"></a>

## Function `assert_collection_exists`



<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address: <b>address</b>, collection_name: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_collection_data &#61; &amp;<b>borrow_global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_collection_data, collection_name), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_assert_tokendata_exists"></a>

## Function `assert_tokendata_exists`



<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> &#123;<br />    <b>let</b> creator_addr &#61; token_data_id.creator;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator) &#61;&#61; creator_addr, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>));<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));<br />    <b>let</b> all_token_data &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br />    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(all_token_data, token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_assert_non_standard_reserved_property"></a>

## Function `assert_non_standard_reserved_property`



<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(keys: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(keys: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;) &#123;<br />    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(keys, &#124;key&#124; &#123;<br />        <b>let</b> key: &amp;String &#61; key;<br />        <b>let</b> length &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_length">string::length</a>(key);<br />        <b>if</b> (length &gt;&#61; 6) &#123;<br />            <b>let</b> prefix &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_sub_string">string::sub_string</a>(&amp;&#42;key, 0, 6);<br />            <b>assert</b>!(prefix !&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;TOKEN_&quot;), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EPROPERTY_RESERVED_BY_STANDARD">EPROPERTY_RESERVED_BY_STANDARD</a>));<br />        &#125;;<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_initialize_token_script"></a>

## Function `initialize_token_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_initialize_token_script">initialize_token_script</a>(_account: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_initialize_token_script">initialize_token_script</a>(_account: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <b>abort</b> 0<br />&#125;<br /></code></pre>



</details>

<a id="0x3_token_initialize_token"></a>

## Function `initialize_token`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token">initialize_token</a>(_account: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token">initialize_token</a>(_account: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>) &#123;<br />    <b>abort</b> 0<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_create_collection_script"></a>

### Function `create_collection_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_collection_script">create_collection_script</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)<br /></code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
The length of the uri is up to MAX_URI_LENGTH;


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="token.md#0x3_token_CreateCollectionAbortsIf">CreateCollectionAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_create_token_script"></a>

### Function `create_token_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_token_script">create_token_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, balance: u64, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>


the length of &apos;mutate_setting&apos; should maore than five.
The creator of the TokenDataId is signer.
The token_data_id should exist in the creator&apos;s collections..
The sum of supply and mint Token is less than maximum.


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> token_data_id &#61; <a href="token.md#0x3_token_spec_create_tokendata">spec_create_tokendata</a>(addr, collection, name);<br /><b>let</b> creator_addr &#61; token_data_id.creator;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> token_data_id.creator !&#61; addr;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>aborts_if</b> balance &lt;&#61; 0;<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a>;<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_spec_create_tokendata"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_create_tokendata">spec_create_tokendata</a>(<br />   creator: <b>address</b>,<br />   collection: String,<br />   name: String): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123;<br />   <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123; creator, collection, name &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_mint_script"></a>

### Function `mint_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mint_script">mint_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, amount: u64)<br /></code></pre>


only creator of the tokendata can mint tokens


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> token_data_id &#61; <a href="token.md#0x3_token_spec_create_token_data_id">spec_create_token_data_id</a>(<br />    token_data_address,<br />    collection,<br />    name,<br />);<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> creator_addr &#61; token_data_id.creator;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> token_data_id.creator !&#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>&#123;<br />creator: token_data_address,<br />collection: collection,<br />name: name<br />&#125;;<br /><b>include</b> <a href="token.md#0x3_token_MintTokenAbortsIf">MintTokenAbortsIf</a> &#123;<br />token_data_id: token_data_id<br />&#125;;<br /></code></pre>



<a id="@Specification_1_mutate_token_properties"></a>

### Function `mutate_token_properties`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mutate_token_properties">mutate_token_properties</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, amount: u64, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>


The signer is creator.


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>aborts_if</b> addr !&#61; creator;<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> &#123;<br />    creator: creator,<br />    collection: collection_name,<br />    name: token_name<br />&#125;;<br /></code></pre>



<a id="@Specification_1_direct_transfer_script"></a>

### Function `direct_transfer_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_direct_transfer_script">direct_transfer_script</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>&#123;<br />    creator: creators_address,<br />    collection: collection,<br />    name: name<br />&#125;;<br /></code></pre>



<a id="@Specification_1_opt_in_direct_transfer"></a>

### Function `opt_in_direct_transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_opt_in_direct_transfer">opt_in_direct_transfer</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> account_addr &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt; MAX_U64;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 9 &gt; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 9 &gt; MAX_U64;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /></code></pre>



<a id="@Specification_1_transfer_with_opt_in"></a>

### Function `transfer_with_opt_in`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_transfer_with_opt_in">transfer_with_opt_in</a>(from: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>&#123;<br />    creator: creator,<br />    collection: collection_name,<br />    name: token_name<br />&#125;;<br /></code></pre>



<a id="@Specification_1_burn_by_creator"></a>

### Function `burn_by_creator`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn_by_creator">burn_by_creator</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> creator_address &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token_spec_create_token_id_raw">spec_create_token_id_raw</a>(creator_address, collection, name, property_version);<br /><b>let</b> creator_addr &#61; token_id.token_data_id.creator;<br /><b>let</b> collections &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<br />    collections.token_data,<br />    token_id.token_data_id,<br />);<br /><b>aborts_if</b> amount &lt;&#61; 0;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.token_data, token_id.token_data_id);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(token_data.default_properties.map, std::string::spec_utf8(<a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>));<br /></code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn">burn</a>(owner: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)<br /></code></pre>


The token_data_id should exist in token_data.


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token_spec_create_token_id_raw">spec_create_token_id_raw</a>(creators_address, collection, name, property_version);<br /><b>let</b> creator_addr &#61; token_id.token_data_id.creator;<br /><b>let</b> collections &#61; <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<br />    collections.token_data,<br />    token_id.token_data_id,<br />);<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> &#123;<br />creator: creators_address<br />&#125;;<br /><b>aborts_if</b> amount &lt;&#61; 0;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.token_data, token_id.token_data_id);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(token_data.default_properties.map, std::string::spec_utf8(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>));<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>);<br /></code></pre>




<a id="0x3_token_spec_create_token_id_raw"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_create_token_id_raw">spec_create_token_id_raw</a>(<br />   creator: <b>address</b>,<br />   collection: String,<br />   name: String,<br />   property_version: u64,<br />): <a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />   <b>let</b> token_data_id &#61; <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123; creator, collection, name &#125;;<br />   <a href="token.md#0x3_token_TokenId">TokenId</a> &#123;<br />       token_data_id,<br />       property_version<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_mutate_collection_description"></a>

### Function `mutate_collection_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_description">mutate_collection_description</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>


The description of Collection is mutable.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data, collection_name);<br /><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> &#123;<br />    creator_address: addr,<br />    collection_name: collection_name<br />&#125;;<br /><b>aborts_if</b> !collection_data.mutability_config.description;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_mutate_collection_uri"></a>

### Function `mutate_collection_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_uri">mutate_collection_uri</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>


The uri of Collection is mutable.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data, collection_name);<br /><b>aborts_if</b> len(uri.bytes) &gt; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>;<br /><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> &#123;<br />    creator_address: addr,<br />    collection_name: collection_name<br />&#125;;<br /><b>aborts_if</b> !collection_data.mutability_config.uri;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_mutate_collection_maximum"></a>

### Function `mutate_collection_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_maximum">mutate_collection_maximum</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64)<br /></code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.
The maximum should more than suply.
The maxium of Collection is mutable.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> collection_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data, collection_name);<br /><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> &#123;<br />    creator_address: addr,<br />    collection_name: collection_name<br />&#125;;<br /><b>aborts_if</b> collection_data.maximum &#61;&#61; 0 &#124;&#124; maximum &#61;&#61; 0;<br /><b>aborts_if</b> maximum &lt; collection_data.supply;<br /><b>aborts_if</b> !collection_data.mutability_config.maximum;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_mutate_tokendata_maximum"></a>

### Function `mutate_tokendata_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_maximum">mutate_tokendata_maximum</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, maximum: u64)<br /></code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.
The maximum should more than suply.
The token maximum is mutable


<pre><code><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;<br /><b>aborts_if</b> token_data.maximum &#61;&#61; 0 &#124;&#124; maximum &#61;&#61; 0;<br /><b>aborts_if</b> maximum &lt; token_data.supply;<br /><b>aborts_if</b> !token_data.mutability_config.maximum;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_mutate_tokendata_uri"></a>

### Function `mutate_tokendata_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_uri">mutate_tokendata_uri</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>


The length of uri should less than MAX_URI_LENGTH.
The  creator of token_data_id should exist in Collections.
The token uri is mutable


<pre><code><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;<br /><b>aborts_if</b> len(uri.bytes) &gt; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>;<br /><b>aborts_if</b> !token_data.mutability_config.uri;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_mutate_tokendata_royalty"></a>

### Function `mutate_tokendata_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_royalty">mutate_tokendata_royalty</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, royalty: <a href="token.md#0x3_token_Royalty">token::Royalty</a>)<br /></code></pre>


The token royalty is mutable


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> !token_data.mutability_config.royalty;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_mutate_tokendata_description"></a>

### Function `mutate_tokendata_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_description">mutate_tokendata_description</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>


The token description is mutable


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> !token_data.mutability_config.description;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token_event_store.md#0x3_token_event_store_TokenEventStoreV1">token_event_store::TokenEventStoreV1</a>&gt;(addr) &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 9 &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_mutate_tokendata_property"></a>

### Function `mutate_tokendata_property`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_property">mutate_tokendata_property</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>


The property map is mutable


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;<br /><b>aborts_if</b> len(keys) !&#61; len(values);<br /><b>aborts_if</b> len(keys) !&#61; len(types);<br /><b>aborts_if</b> !token_data.mutability_config.properties;<br /></code></pre>



<a id="@Specification_1_mutate_one_token"></a>

### Function `mutate_one_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>


The signer is creator.
The token_data_id should exist in token_data.
The property map is mutable.


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> creator &#61; token_id.token_data_id.creator;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_id.token_data_id);<br /><b>aborts_if</b> addr !&#61; creator;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_id.token_data_id);<br /><b>aborts_if</b> !token_data.mutability_config.properties &amp;&amp; !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(token_data.default_properties.map, std::string::spec_utf8(<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>));<br /></code></pre>



<a id="@Specification_1_create_royalty"></a>

### Function `create_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: <b>address</b>): <a href="token.md#0x3_token_Royalty">token::Royalty</a><br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_CreateRoyaltyAbortsIf">CreateRoyaltyAbortsIf</a>;<br /></code></pre>


The royalty_points_numerator should less than royalty_points_denominator.


<a id="0x3_token_CreateRoyaltyAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateRoyaltyAbortsIf">CreateRoyaltyAbortsIf</a> &#123;<br />royalty_points_numerator: u64;<br />royalty_points_denominator: u64;<br />payee_address: <b>address</b>;<br /><b>aborts_if</b> royalty_points_numerator &gt; royalty_points_denominator;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(payee_address);<br />&#125;<br /></code></pre>



<a id="@Specification_1_deposit_token"></a>

### Function `deposit_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>include</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr) &#61;&#61;&gt; <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a>;<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token">token</a>.id;<br /><b>let</b> token_amount &#61; <a href="token.md#0x3_token">token</a>.amount;<br /><b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_direct_deposit_with_opt_in"></a>

### Function `direct_deposit_with_opt_in`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_deposit_with_opt_in">direct_deposit_with_opt_in</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>


The token can direct_transfer.


<pre><code><b>let</b> opt_in_transfer &#61; <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr).direct_transfer;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);<br /><b>aborts_if</b> !opt_in_transfer;<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token">token</a>.id;<br /><b>let</b> token_amount &#61; <a href="token.md#0x3_token">token</a>.amount;<br /><b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_direct_transfer"></a>

### Function `direct_transfer`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(sender: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)<br /></code></pre>


Cannot withdraw 0 tokens.
Make sure the account has sufficient tokens to withdraw.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_initialize_token_store"></a>

### Function `initialize_token_store`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a>;<br /></code></pre>




<a id="0x3_token_InitializeTokenStore"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a> &#123;<br /><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> account_addr &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(addr) &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt; MAX_U64;<br />&#125;<br /></code></pre>



<a id="@Specification_1_merge"></a>

### Function `merge`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_merge">merge</a>(dst_token: &amp;<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, source_token: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> dst_token.id !&#61; source_token.id;<br /><b>aborts_if</b> dst_token.amount &#43; source_token.amount &gt; MAX_U64;<br /></code></pre>



<a id="@Specification_1_split"></a>

### Function `split`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_split">split</a>(dst_token: &amp;<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>




<pre><code><b>aborts_if</b> dst_token.id.property_version !&#61; 0;<br /><b>aborts_if</b> dst_token.amount &lt;&#61; amount;<br /><b>aborts_if</b> amount &lt;&#61; 0;<br /></code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_transfer">transfer</a>(from: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, <b>to</b>: <b>address</b>, amount: u64)<br /></code></pre>




<pre><code><b>let</b> opt_in_transfer &#61; <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(<b>to</b>).direct_transfer;<br /><b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);<br /><b>aborts_if</b> !opt_in_transfer;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_withdraw_with_capability"></a>

### Function `withdraw_with_capability`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_with_capability">withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>




<pre><code><b>let</b> now_seconds &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework).microseconds;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> now_seconds / <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">timestamp::MICRO_CONVERSION_FACTOR</a> &gt; withdraw_proof.expiration_sec;<br /><b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>&#123;<br />account_addr: withdraw_proof.token_owner,<br />id: withdraw_proof.token_id,<br />amount: withdraw_proof.amount&#125;;<br /></code></pre>



<a id="@Specification_1_partial_withdraw_with_capability"></a>

### Function `partial_withdraw_with_capability`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_partial_withdraw_with_capability">partial_withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>, withdraw_amount: u64): (<a href="token.md#0x3_token_Token">token::Token</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>&gt;)<br /></code></pre>




<pre><code><b>let</b> now_seconds &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework).microseconds;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> now_seconds / <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">timestamp::MICRO_CONVERSION_FACTOR</a> &gt; withdraw_proof.expiration_sec;<br /><b>aborts_if</b> withdraw_amount &gt; withdraw_proof.amount;<br /><b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>&#123;<br />    account_addr: withdraw_proof.token_owner,<br />    id: withdraw_proof.token_id,<br />    amount: withdraw_amount<br />&#125;;<br /></code></pre>



<a id="@Specification_1_withdraw_token"></a>

### Function `withdraw_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>


Cannot withdraw 0 tokens.
Make sure the account has sufficient tokens to withdraw.


<pre><code><b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_create_collection"></a>

### Function `create_collection`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection">create_collection</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)<br /></code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
The length of the uri is up to MAX_URI_LENGTH;
The collection_data should not exist before you create it.


<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>aborts_if</b> len(name.bytes) &gt; 128;<br /><b>aborts_if</b> len(uri.bytes) &gt; 512;<br /><b>include</b> <a href="token.md#0x3_token_CreateCollectionAbortsIf">CreateCollectionAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_CreateCollectionAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateCollectionAbortsIf">CreateCollectionAbortsIf</a> &#123;<br />creator: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />name: String;<br />description: String;<br />uri: String;<br />maximum: u64;<br />mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>let</b> collection &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr);<br /><b>let</b> b &#61; !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr);<br /><b>let</b> collection_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data;<br /><b>aborts_if</b> b &amp;&amp; !<b>exists</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);<br /><b>aborts_if</b> len(name.bytes) &gt; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>;<br /><b>aborts_if</b> len(uri.bytes) &gt; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>;<br /><b>aborts_if</b> b &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 3 &gt;&#61; <a href="../../aptos-framework/doc/account.md#0x1_account_MAX_GUID_CREATION_NUM">account::MAX_GUID_CREATION_NUM</a>;<br /><b>aborts_if</b> b &amp;&amp; <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>.guid_creation_num &#43; 3 &gt; MAX_U64;<br /><b>include</b> <a href="token.md#0x3_token_CreateCollectionMutabilityConfigAbortsIf">CreateCollectionMutabilityConfigAbortsIf</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_check_collection_exists"></a>

### Function `check_collection_exists`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_collection_exists">check_collection_exists</a>(creator: <b>address</b>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);<br /></code></pre>



<a id="@Specification_1_check_tokendata_exists"></a>

### Function `check_tokendata_exists`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_tokendata_exists">check_tokendata_exists</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool<br /></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> &#123;<br />    creator: creator,<br />    collection: collection_name,<br />    name: token_name<br />&#125;;<br /></code></pre>



<a id="@Specification_1_create_tokendata"></a>

### Function `create_tokendata`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a><br /></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> account_addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> collections &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr);<br /><b>let</b> token_data_id &#61; <a href="token.md#0x3_token_spec_create_token_data_id">spec_create_token_data_id</a>(account_addr, collection, name);<br /><b>let</b> Collection &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(collections.collection_data, token_data_id.collection);<br /><b>let</b> length &#61; len(property_keys);<br /><b>aborts_if</b> len(name.bytes) &gt; <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>;<br /><b>aborts_if</b> len(collection.bytes) &gt; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>;<br /><b>aborts_if</b> len(uri.bytes) &gt; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>;<br /><b>aborts_if</b> royalty_points_numerator &gt; royalty_points_denominator;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr);<br /><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> &#123;<br />    creator: account_addr,<br />    collection: collection,<br />    name: name<br />&#125;;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.collection_data, collection);<br /><b>aborts_if</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.token_data, token_data_id);<br /><b>aborts_if</b> Collection.maximum &gt; 0 &amp;&amp; Collection.supply &#43; 1 &gt; MAX_U64;<br /><b>aborts_if</b> Collection.maximum &gt; 0 &amp;&amp; Collection.maximum &lt; Collection.supply &#43; 1;<br /><b>include</b> <a href="token.md#0x3_token_CreateRoyaltyAbortsIf">CreateRoyaltyAbortsIf</a> &#123;<br />    payee_address: royalty_payee_address<br />&#125;;<br /><b>aborts_if</b> length &gt; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">property_map::MAX_PROPERTY_MAP_SIZE</a>;<br /><b>aborts_if</b> length !&#61; len(property_values);<br /><b>aborts_if</b> length !&#61; len(property_types);<br /></code></pre>




<a id="0x3_token_spec_create_token_data_id"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_create_token_data_id">spec_create_token_data_id</a>(<br />   creator: <b>address</b>,<br />   collection: String,<br />   name: String,<br />): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123;<br />   <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> &#123; creator, collection, name &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_collection_supply"></a>

### Function `get_collection_supply`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_supply">get_collection_supply</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_get_collection_description"></a>

### Function `get_collection_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_description">get_collection_description</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_get_collection_uri"></a>

### Function `get_collection_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_uri">get_collection_uri</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_get_collection_maximum"></a>

### Function `get_collection_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_maximum">get_collection_maximum</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64<br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;<br /></code></pre>



<a id="@Specification_1_get_token_supply"></a>

### Function `get_token_supply`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_supply">get_token_supply</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /></code></pre>



<a id="@Specification_1_get_tokendata_largest_property_version"></a>

### Function `get_tokendata_largest_property_version`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_largest_property_version">get_tokendata_largest_property_version</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /></code></pre>



<a id="@Specification_1_create_token_mutability_config"></a>

### Function `create_token_mutability_config`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(mutate_setting: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a><br /></code></pre>


The length of &apos;mutate_setting&apos; should more than five.
The mutate_setting shuold have a value.


<pre><code><b>include</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_CreateTokenMutabilityConfigAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a> &#123;<br />mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;;<br /><b>aborts_if</b> len(mutate_setting) &lt; 5;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_MAX_MUTABLE_IND">TOKEN_MAX_MUTABLE_IND</a>]);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_URI_MUTABLE_IND">TOKEN_URI_MUTABLE_IND</a>]);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_ROYALTY_MUTABLE_IND">TOKEN_ROYALTY_MUTABLE_IND</a>]);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND">TOKEN_DESCRIPTION_MUTABLE_IND</a>]);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE_IND">TOKEN_PROPERTY_MUTABLE_IND</a>]);<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_collection_mutability_config"></a>

### Function `create_collection_mutability_config`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(mutate_setting: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a><br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_CreateCollectionMutabilityConfigAbortsIf">CreateCollectionMutabilityConfigAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_CreateCollectionMutabilityConfigAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateCollectionMutabilityConfigAbortsIf">CreateCollectionMutabilityConfigAbortsIf</a> &#123;<br />mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;;<br /><b>aborts_if</b> len(mutate_setting) &lt; 3;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND">COLLECTION_DESCRIPTION_MUTABLE_IND</a>]);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_COLLECTION_URI_MUTABLE_IND">COLLECTION_URI_MUTABLE_IND</a>]);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_COLLECTION_MAX_MUTABLE_IND">COLLECTION_MAX_MUTABLE_IND</a>]);<br />&#125;<br /></code></pre>



<a id="@Specification_1_mint_token"></a>

### Function `mint_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token">mint_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>


The creator of the TokenDataId is signer.
The token_data_id should exist in the creator&apos;s collections..
The sum of supply and the amount of mint Token is less than maximum.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>




<a id="0x3_token_MintTokenAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_MintTokenAbortsIf">MintTokenAbortsIf</a> &#123;<br /><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>;<br />amount: u64;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> creator_addr &#61; token_data_id.creator;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> token_data_id.creator !&#61; addr;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> token_data.maximum &gt; 0 &amp;&amp; token_data.supply &#43; amount &gt; token_data.maximum;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>aborts_if</b> amount &lt;&#61; 0;<br /><b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a>;<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);<br />&#125;<br /></code></pre>



<a id="@Specification_1_mint_token_to"></a>

### Function `mint_token_to`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token_to">mint_token_to</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);<br /><b>let</b> opt_in_transfer &#61; <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver).direct_transfer;<br /><b>let</b> creator_addr &#61; token_data_id.creator;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br /><b>let</b> token_data &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver);<br /><b>aborts_if</b> !opt_in_transfer;<br /><b>aborts_if</b> token_data_id.creator !&#61; addr;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /><b>aborts_if</b> token_data.maximum &gt; 0 &amp;&amp; token_data.supply &#43; amount &gt; token_data.maximum;<br /><b>aborts_if</b> amount &lt;&#61; 0;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>let</b> token_id &#61; <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);<br /><b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a> &#123;<br />    account_addr: receiver,<br />    token_id: token_id,<br />    token_amount: amount,<br />&#125;;<br /></code></pre>



<a id="@Specification_1_create_token_data_id"></a>

### Function `create_token_data_id`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a><br /></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_CreateTokenDataIdAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> &#123;<br />creator: <b>address</b>;<br />collection: String;<br />name: String;<br /><b>aborts_if</b> len(collection.bytes) &gt; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>;<br /><b>aborts_if</b> len(name.bytes) &gt; <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_token_id_raw"></a>

### Function `create_token_id_raw`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a><br /></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_spec_balance_of"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_balance_of">spec_balance_of</a>(owner: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">TokenId</a>): u64 &#123;<br />   <b>let</b> token_store &#61; <b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner);<br />   <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner)) &#123;<br />       0<br />   &#125;<br />   <b>else</b> <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, id)) &#123;<br />       <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, id).amount<br />   &#125; <b>else</b> &#123;<br />       0<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_royalty"></a>

### Function `get_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty">get_royalty</a>(token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a><br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_GetTokendataRoyaltyAbortsIf">GetTokendataRoyaltyAbortsIf</a> &#123;<br />    token_data_id: token_id.token_data_id<br />&#125;;<br /></code></pre>



<a id="@Specification_1_get_property_map"></a>

### Function `get_property_map`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_property_map">get_property_map</a>(owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a><br /></code></pre>




<pre><code><b>let</b> creator_addr &#61; token_id.token_data_id.creator;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br /><b>aborts_if</b> <a href="token.md#0x3_token_spec_balance_of">spec_balance_of</a>(owner, token_id) &lt;&#61; 0;<br /><b>aborts_if</b> token_id.property_version &#61;&#61; 0 &amp;&amp; !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_id.token_data_id);<br /><b>aborts_if</b> token_id.property_version &#61;&#61; 0 &amp;&amp; !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /></code></pre>



<a id="@Specification_1_get_tokendata_maximum"></a>

### Function `get_tokendata_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_maximum">get_tokendata_maximum</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64<br /></code></pre>




<pre><code><b>let</b> creator_address &#61; token_data_id.creator;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /></code></pre>



<a id="@Specification_1_get_tokendata_uri"></a>

### Function `get_tokendata_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_uri">get_tokendata_uri</a>(creator: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).token_data;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /></code></pre>



<a id="@Specification_1_get_tokendata_description"></a>

### Function `get_tokendata_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_description">get_tokendata_description</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a><br /></code></pre>




<pre><code><b>let</b> creator_address &#61; token_data_id.creator;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /></code></pre>



<a id="@Specification_1_get_tokendata_royalty"></a>

### Function `get_tokendata_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a><br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_GetTokendataRoyaltyAbortsIf">GetTokendataRoyaltyAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_GetTokendataRoyaltyAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_GetTokendataRoyaltyAbortsIf">GetTokendataRoyaltyAbortsIf</a> &#123;<br />token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>;<br /><b>let</b> creator_address &#61; token_data_id.creator;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_tokendata_mutability_config"></a>

### Function `get_tokendata_mutability_config`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_mutability_config">get_tokendata_mutability_config</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a><br /></code></pre>




<pre><code><b>let</b> creator_addr &#61; token_data_id.creator;<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br /></code></pre>



<a id="@Specification_1_get_collection_mutability_config"></a>

### Function `get_collection_mutability_config`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_config">get_collection_mutability_config</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a><br /></code></pre>




<pre><code><b>let</b> all_collection_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).collection_data;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_collection_data, collection_name);<br /></code></pre>



<a id="@Specification_1_withdraw_with_event_internal"></a>

### Function `withdraw_with_event_internal`


<pre><code><b>fun</b> <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(account_addr: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a><br /></code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_WithdrawWithEventInternalAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a> &#123;<br />account_addr: <b>address</b>;<br />id: <a href="token.md#0x3_token_TokenId">TokenId</a>;<br />amount: u64;<br /><b>let</b> tokens &#61; <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr).tokens;<br /><b>aborts_if</b> amount &lt;&#61; 0;<br /><b>aborts_if</b> <a href="token.md#0x3_token_spec_balance_of">spec_balance_of</a>(account_addr, id) &lt; amount;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(tokens, id);<br />&#125;<br /></code></pre>



<a id="@Specification_1_update_token_property_internal"></a>

### Function `update_token_property_internal`


<pre><code><b>fun</b> <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> tokens &#61; <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(token_owner).tokens;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(token_owner);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(tokens, token_id);<br /></code></pre>



<a id="@Specification_1_direct_deposit"></a>

### Function `direct_deposit`


<pre><code><b>fun</b> <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)<br /></code></pre>




<pre><code><b>let</b> token_id &#61; <a href="token.md#0x3_token">token</a>.id;<br /><b>let</b> token_amount &#61; <a href="token.md#0x3_token">token</a>.amount;<br /><b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_DirectDepositAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a> &#123;<br />account_addr: <b>address</b>;<br />token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>;<br />token_amount: u64;<br /><b>let</b> token_store &#61; <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);<br /><b>let</b> recipient_token &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, token_id);<br /><b>let</b> b &#61; <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, token_id);<br /><b>aborts_if</b> token_amount &lt;&#61; 0;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);<br /><b>aborts_if</b> b &amp;&amp; recipient_token.id !&#61; token_id;<br /><b>aborts_if</b> b &amp;&amp; recipient_token.amount &#43; token_amount &gt; MAX_U64;<br />&#125;<br /></code></pre>



<a id="@Specification_1_assert_collection_exists"></a>

### Function `assert_collection_exists`


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>


The collection_name should exist in collection_data of the creator_address&apos;s Collections.


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_AssertCollectionExistsAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> &#123;<br />creator_address: <b>address</b>;<br />collection_name: String;<br /><b>let</b> all_collection_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_collection_data, collection_name);<br />&#125;<br /></code></pre>



<a id="@Specification_1_assert_tokendata_exists"></a>

### Function `assert_tokendata_exists`


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>)<br /></code></pre>


The creator of token_data_id should be signer.
The  creator of token_data_id exists in Collections.
The token_data_id is in the all_token_data.


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;<br /></code></pre>




<a id="0x3_token_AssertTokendataExistsAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a> &#123;<br />creator: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>;<br /><b>let</b> creator_addr &#61; token_data_id.creator;<br /><b>let</b> addr &#61; <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);<br /><b>aborts_if</b> addr !&#61; creator_addr;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);<br /><b>let</b> all_token_data &#61; <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;<br /><b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);<br />&#125;<br /></code></pre>



<a id="@Specification_1_assert_non_standard_reserved_property"></a>

### Function `assert_non_standard_reserved_property`


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(keys: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_initialize_token_script"></a>

### Function `initialize_token_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_initialize_token_script">initialize_token_script</a>(_account: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


Deprecated function


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_initialize_token"></a>

### Function `initialize_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token">initialize_token</a>(_account: &amp;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)<br /></code></pre>


Deprecated function


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY

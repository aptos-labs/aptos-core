// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_db_indexer_schemas::schema::event_sequence_number::EventSequenceNumberSchema;
use aptos_schemadb::DB;
use aptos_storage_interface::{
    AptosDbError, DbReader, Result,
    state_store::state_view::db_state_view::LatestDbStateCheckpointView,
};
use aptos_types::{
    DummyCoinType,
    account_config::{
        AccountResource, BURN_EVENT_TYPE, BURN_TOKEN_EVENT_TYPE, BURN_TOKEN_TYPE, BURN_TYPE, Burn,
        BurnEvent, BurnToken, BurnTokenEvent, CANCEL_OFFER_TYPE, CLAIM_TYPE, COIN_DEPOSIT_TYPE,
        COIN_REGISTER_EVENT_TYPE, COIN_REGISTER_TYPE, COIN_WITHDRAW_TYPE,
        COLLECTION_DESCRIPTION_MUTATE_EVENT_TYPE, COLLECTION_DESCRIPTION_MUTATE_TYPE,
        COLLECTION_MAXIMUM_MUTATE_EVENT_TYPE, COLLECTION_MAXIMUM_MUTATE_TYPE,
        COLLECTION_MUTATION_EVENT_TYPE, COLLECTION_MUTATION_TYPE, COLLECTION_URI_MUTATE_EVENT_TYPE,
        COLLECTION_URI_MUTATE_TYPE, CREATE_COLLECTION_EVENT_TYPE, CREATE_COLLECTION_TYPE,
        CREATE_TOKEN_DATA_EVENT_TYPE, CancelOffer, Claim, CoinDeposit, CoinRegister,
        CoinRegisterEvent, CoinStoreResource, CoinWithdraw, CollectionDescriptionMutate,
        CollectionDescriptionMutateEvent, CollectionMaximumMutate, CollectionMaximumMutateEvent,
        CollectionMutation, CollectionMutationEvent, CollectionResource, CollectionUriMutate,
        CollectionUriMutateEvent, CollectionsResource, CreateCollection, CreateCollectionEvent,
        CreateTokenDataEvent, DEFAULT_PROPERTY_MUTATE_EVENT_TYPE, DEFAULT_PROPERTY_MUTATE_TYPE,
        DEPOSIT_EVENT_TYPE, DESCRIPTION_MUTATE_EVENT_TYPE, DESCRIPTION_MUTATE_TYPE,
        DefaultPropertyMutate, DefaultPropertyMutateEvent, DepositEvent, DescriptionMutate,
        DescriptionMutateEvent, FixedSupplyResource, KEY_ROTATION_EVENT_TYPE, KEY_ROTATION_TYPE,
        KeyRotation, KeyRotationEvent, MAXIMUM_MUTATE_EVENT_TYPE, MAXIMUM_MUTATE_TYPE,
        MINT_EVENT_TYPE, MINT_TOKEN_EVENT_TYPE, MINT_TOKEN_TYPE, MINT_TYPE,
        MUTATE_PROPERTY_MAP_TYPE, MUTATE_TOKEN_PROPERTY_MAP_EVENT_TYPE, MaximumMutate,
        MaximumMutateEvent, Mint, MintEvent, MintToken, MintTokenEvent, MutatePropertyMap,
        MutateTokenPropertyMapEvent, OFFER_TYPE, OPT_IN_TRANSFER_EVENT_TYPE, OPT_IN_TRANSFER_TYPE,
        ObjectCoreResource, ObjectGroupResource, Offer, OptInTransfer, OptInTransferEvent,
        PendingClaimsResource, ROYALTY_MUTATE_EVENT_TYPE, ROYALTY_MUTATE_TYPE, RoyaltyMutate,
        RoyaltyMutateEvent, TOKEN_CANCEL_OFFER_EVENT_TYPE, TOKEN_CLAIM_EVENT_TYPE,
        TOKEN_DATA_CREATION_TYPE, TOKEN_DEPOSIT_EVENT_TYPE, TOKEN_DEPOSIT_TYPE,
        TOKEN_MUTATION_EVENT_TYPE, TOKEN_MUTATION_TYPE, TOKEN_OFFER_EVENT_TYPE,
        TOKEN_WITHDRAW_EVENT_TYPE, TOKEN_WITHDRAW_TYPE, TRANSFER_EVENT_TYPE, TRANSFER_TYPE,
        TokenCancelOfferEvent, TokenClaimEvent, TokenDataCreation, TokenDeposit, TokenDepositEvent,
        TokenEventStoreV1Resource, TokenMutation, TokenMutationEvent, TokenOfferEvent,
        TokenResource, TokenStoreResource, TokenWithdraw, TokenWithdrawEvent, Transfer,
        TransferEvent, URI_MUTATION_EVENT_TYPE, URI_MUTATION_TYPE, UnlimitedSupplyResource,
        UriMutation, UriMutationEvent, WITHDRAW_EVENT_TYPE, WithdrawEvent,
    },
    contract_event::{ContractEventV1, ContractEventV2},
    event::EventKey,
    state_store::{TStateView, state_key::StateKey},
};
use bytes::Bytes;
use dashmap::DashMap;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use std::{collections::HashMap, str::FromStr, sync::Arc};

pub trait EventV2Translator: Send + Sync {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1>;
}

pub struct EventV2TranslationEngine {
    pub main_db_reader: Arc<dyn DbReader>,
    pub internal_indexer_db: Arc<DB>,
    // Map from event type to translator
    pub translators: HashMap<TypeTag, Box<dyn EventV2Translator + Send + Sync>>,
    event_sequence_number_cache: DashMap<EventKey, u64>,
}

impl EventV2TranslationEngine {
    pub fn new(main_db_reader: Arc<dyn DbReader>, internal_indexer_db: Arc<DB>) -> Self {
        let translators: HashMap<TypeTag, Box<dyn EventV2Translator + Send + Sync>> = [
            (
                COIN_DEPOSIT_TYPE.clone(),
                Box::new(CoinDepositTranslator) as Box<dyn EventV2Translator + Send + Sync>,
            ),
            (COIN_WITHDRAW_TYPE.clone(), Box::new(CoinWithdrawTranslator)),
            (COIN_REGISTER_TYPE.clone(), Box::new(CoinRegisterTranslator)),
            (KEY_ROTATION_TYPE.clone(), Box::new(KeyRotationTranslator)),
            (TRANSFER_TYPE.clone(), Box::new(TransferTranslator)),
            (
                TOKEN_MUTATION_TYPE.clone(),
                Box::new(TokenMutationTranslator),
            ),
            (
                COLLECTION_MUTATION_TYPE.clone(),
                Box::new(CollectionMutationTranslator),
            ),
            (MINT_TYPE.clone(), Box::new(MintTranslator)),
            (BURN_TYPE.clone(), Box::new(BurnTranslator)),
            (TOKEN_DEPOSIT_TYPE.clone(), Box::new(TokenDepositTranslator)),
            (
                TOKEN_WITHDRAW_TYPE.clone(),
                Box::new(TokenWithdrawTranslator),
            ),
            (BURN_TOKEN_TYPE.clone(), Box::new(BurnTokenTranslator)),
            (
                MUTATE_PROPERTY_MAP_TYPE.clone(),
                Box::new(MutatePropertyMapTranslator),
            ),
            (MINT_TOKEN_TYPE.clone(), Box::new(MintTokenTranslator)),
            (
                CREATE_COLLECTION_TYPE.clone(),
                Box::new(CreateCollectionTranslator),
            ),
            (
                TOKEN_DATA_CREATION_TYPE.clone(),
                Box::new(TokenDataCreationTranslator),
            ),
            (OFFER_TYPE.clone(), Box::new(OfferTranslator)),
            (CANCEL_OFFER_TYPE.clone(), Box::new(CancelOfferTranslator)),
            (CLAIM_TYPE.clone(), Box::new(ClaimTranslator)),
            (
                COLLECTION_DESCRIPTION_MUTATE_TYPE.clone(),
                Box::new(CollectionDescriptionMutateTranslator),
            ),
            (
                COLLECTION_URI_MUTATE_TYPE.clone(),
                Box::new(CollectionUriMutateTranslator),
            ),
            (
                COLLECTION_MAXIMUM_MUTATE_TYPE.clone(),
                Box::new(CollectionMaximumMutateTranslator),
            ),
            (URI_MUTATION_TYPE.clone(), Box::new(UriMutationTranslator)),
            (
                DEFAULT_PROPERTY_MUTATE_TYPE.clone(),
                Box::new(DefaultPropertyMutateTranslator),
            ),
            (
                DESCRIPTION_MUTATE_TYPE.clone(),
                Box::new(DescriptionMutateTranslator),
            ),
            (
                ROYALTY_MUTATE_TYPE.clone(),
                Box::new(RoyaltyMutateTranslator),
            ),
            (
                MAXIMUM_MUTATE_TYPE.clone(),
                Box::new(MaximumMutateTranslator),
            ),
            (
                OPT_IN_TRANSFER_TYPE.clone(),
                Box::new(OptInTransferTranslator),
            ),
        ]
        .into_iter()
        .collect();
        Self {
            main_db_reader,
            internal_indexer_db,
            translators,
            event_sequence_number_cache: DashMap::new(),
        }
    }

    // When the node starts with a non-empty EventSequenceNumberSchema table, the in-memory cache
    // `event_sequence_number_cache` is empty. In the future, we decide to backup and restore the
    // event sequence number data to support fast sync, we may need to load the cache from the DB
    // when the node starts using this function `load_cache_from_db`.
    pub fn load_cache_from_db(&self) -> Result<()> {
        let mut iter = self
            .internal_indexer_db
            .iter::<EventSequenceNumberSchema>()?;
        iter.seek_to_first();
        while let Some((event_key, sequence_number)) = iter.next().transpose()? {
            self.event_sequence_number_cache
                .insert(event_key, sequence_number);
        }
        Ok(())
    }

    pub fn cache_sequence_number(&self, event_key: &EventKey, sequence_number: u64) {
        self.event_sequence_number_cache
            .insert(*event_key, sequence_number);
    }

    pub fn get_cached_sequence_number(&self, event_key: &EventKey) -> Option<u64> {
        self.event_sequence_number_cache
            .get(event_key)
            .map(|seq| *seq)
    }

    pub fn get_next_sequence_number(&self, event_key: &EventKey, default: u64) -> Result<u64> {
        if let Some(seq) = self.get_cached_sequence_number(event_key) {
            Ok(seq + 1)
        } else {
            let seq = self
                .internal_indexer_db
                .get::<EventSequenceNumberSchema>(event_key)?
                .map_or(default, |seq| seq + 1);
            Ok(seq)
        }
    }

    pub fn get_state_value_bytes_for_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Bytes>> {
        let state_view = self
            .main_db_reader
            .latest_state_checkpoint_view()
            .expect("Failed to get state view");
        let state_key = StateKey::resource(address, struct_tag)?;
        let maybe_state_value = state_view.get_state_value(&state_key)?;
        Ok(maybe_state_value.map(|state_value| state_value.bytes().clone()))
    }

    pub fn get_state_value_bytes_for_object_group_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Bytes>> {
        let state_view = self
            .main_db_reader
            .latest_state_checkpoint_view()
            .expect("Failed to get state view");
        static OBJECT_GROUP_TAG: Lazy<StructTag> = Lazy::new(ObjectGroupResource::struct_tag);
        let state_key = StateKey::resource_group(address, &OBJECT_GROUP_TAG);
        let maybe_state_value = state_view.get_state_value(&state_key)?;
        let state_value = maybe_state_value
            .ok_or_else(|| anyhow::format_err!("ObjectGroup resource not found"))?;
        let object_group_resource: ObjectGroupResource = bcs::from_bytes(state_value.bytes())?;
        Ok(object_group_resource
            .group
            .get(struct_tag)
            .map(|bytes| Bytes::copy_from_slice(bytes)))
    }
}

struct CoinDepositTranslator;
impl EventV2Translator for CoinDepositTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let coin_deposit = CoinDeposit::try_from_bytes(v2.event_data())?;
        let struct_tag_str = format!("0x1::coin::CoinStore<{}>", coin_deposit.coin_type());
        let struct_tag = StructTag::from_str(&struct_tag_str)?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(coin_deposit.account(), &struct_tag)?
        {
            // We can use `DummyCoinType` as it does not affect the correctness of deserialization.
            let coin_store_resource: CoinStoreResource<DummyCoinType> =
                bcs::from_bytes(&state_value_bytes)?;
            let key = *coin_store_resource.deposit_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, coin_store_resource.deposit_events().count())?;
            (key, sequence_number)
        } else {
            // The creation number of DepositEvent is deterministically 2.
            static DEPOSIT_EVENT_CREATION_NUMBER: u64 = 2;
            (
                EventKey::new(DEPOSIT_EVENT_CREATION_NUMBER, *coin_deposit.account()),
                0,
            )
        };
        let deposit_event = DepositEvent::new(coin_deposit.amount());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            DEPOSIT_EVENT_TYPE.clone(),
            bcs::to_bytes(&deposit_event)?,
        )?)
    }
}

struct CoinWithdrawTranslator;
impl EventV2Translator for CoinWithdrawTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let coin_withdraw = CoinWithdraw::try_from_bytes(v2.event_data())?;
        let struct_tag_str = format!("0x1::coin::CoinStore<{}>", coin_withdraw.coin_type());
        let struct_tag = StructTag::from_str(&struct_tag_str)?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(coin_withdraw.account(), &struct_tag)?
        {
            // We can use `DummyCoinType` as it does not affect the correctness of deserialization.
            let coin_store_resource: CoinStoreResource<DummyCoinType> =
                bcs::from_bytes(&state_value_bytes)?;
            let key = *coin_store_resource.withdraw_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, coin_store_resource.withdraw_events().count())?;
            (key, sequence_number)
        } else {
            // The creation number of WithdrawEvent is deterministically 3.
            static WITHDRAW_EVENT_CREATION_NUMBER: u64 = 3;
            (
                EventKey::new(WITHDRAW_EVENT_CREATION_NUMBER, *coin_withdraw.account()),
                0,
            )
        };
        let withdraw_event = WithdrawEvent::new(coin_withdraw.amount());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            WITHDRAW_EVENT_TYPE.clone(),
            bcs::to_bytes(&withdraw_event)?,
        )?)
    }
}

struct CoinRegisterTranslator;
impl EventV2Translator for CoinRegisterTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let coin_register = CoinRegister::try_from_bytes(v2.event_data())?;
        let struct_tag_str = "0x1::account::Account".to_string();
        let struct_tag = StructTag::from_str(&struct_tag_str)?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(coin_register.account(), &struct_tag)?
        {
            let account_resource: AccountResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *account_resource.coin_register_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, account_resource.coin_register_events().count())?;
            (key, sequence_number)
        } else {
            // The creation number of CoinRegisterEvent is deterministically 0.
            static COIN_REGISTER_EVENT_CREATION_NUMBER: u64 = 0;
            (
                EventKey::new(
                    COIN_REGISTER_EVENT_CREATION_NUMBER,
                    *coin_register.account(),
                ),
                0,
            )
        };
        let coin_register_event = CoinRegisterEvent::new(coin_register.type_info().clone());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            COIN_REGISTER_EVENT_TYPE.clone(),
            bcs::to_bytes(&coin_register_event)?,
        )?)
    }
}

struct KeyRotationTranslator;
impl EventV2Translator for KeyRotationTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let key_rotation = KeyRotation::try_from_bytes(v2.event_data())?;
        let struct_tag_str = "0x1::account::Account".to_string();
        let struct_tag = StructTag::from_str(&struct_tag_str)?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(key_rotation.account(), &struct_tag)?
        {
            let account_resource: AccountResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *account_resource.key_rotation_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, account_resource.key_rotation_events().count())?;
            (key, sequence_number)
        } else {
            // The creation number of KeyRotationEvent is deterministically 1.
            static KEY_ROTATION_EVENT_CREATION_NUMBER: u64 = 1;
            (
                EventKey::new(KEY_ROTATION_EVENT_CREATION_NUMBER, *key_rotation.account()),
                0,
            )
        };
        let key_rotation_event = KeyRotationEvent::new(
            key_rotation.old_authentication_key().clone(),
            key_rotation.new_authentication_key().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            KEY_ROTATION_EVENT_TYPE.clone(),
            bcs::to_bytes(&key_rotation_event)?,
        )?)
    }
}

struct TransferTranslator;
impl EventV2Translator for TransferTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let transfer = Transfer::try_from_bytes(v2.event_data())?;
        let struct_tag_str = "0x1::object::ObjectCore".to_string();
        let struct_tag = StructTag::from_str(&struct_tag_str)?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_object_group_resource(transfer.object(), &struct_tag)?
        {
            let object_core_resource: ObjectCoreResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_core_resource.transfer_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, object_core_resource.transfer_events().count())?;
            (key, sequence_number)
        } else {
            // The creation number of TransferEvent is deterministically 0x4000000000000
            // because the INIT_GUID_CREATION_NUM in the Move module is 0x4000000000000.
            static TRANSFER_EVENT_CREATION_NUMBER: u64 = 0x4000000000000;
            (
                EventKey::new(TRANSFER_EVENT_CREATION_NUMBER, *transfer.object()),
                0,
            )
        };
        let transfer_event =
            TransferEvent::new(*transfer.object(), *transfer.from(), *transfer.to());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            TRANSFER_EVENT_TYPE.clone(),
            bcs::to_bytes(&transfer_event)?,
        )?)
    }
}

struct TokenMutationTranslator;
impl EventV2Translator for TokenMutationTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let token_mutation = TokenMutation::try_from_bytes(v2.event_data())?;
        let struct_tag_str = "0x4::token::Token".to_string();
        let struct_tag = StructTag::from_str(&struct_tag_str)?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_object_group_resource(
                token_mutation.token_address(),
                &struct_tag,
            )? {
            let token_resource: TokenResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *token_resource.mutation_events().key();
            let sequence_number =
                engine.get_next_sequence_number(&key, token_resource.mutation_events().count())?;
            (key, sequence_number)
        } else {
            // If the token resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided. The token may have been burned.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Token resource not found"
            )));
        };
        let token_mutation_event = TokenMutationEvent::new(
            token_mutation.mutated_field_name().clone(),
            token_mutation.old_value().clone(),
            token_mutation.new_value().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            TOKEN_MUTATION_EVENT_TYPE.clone(),
            bcs::to_bytes(&token_mutation_event)?,
        )?)
    }
}

struct CollectionMutationTranslator;
impl EventV2Translator for CollectionMutationTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let collection_mutation = CollectionMutation::try_from_bytes(v2.event_data())?;
        let struct_tag_str = "0x4::collection::Collection".to_string();
        let struct_tag = StructTag::from_str(&struct_tag_str)?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_object_group_resource(
                collection_mutation.collection().inner(),
                &struct_tag,
            )? {
            let collection_resource: CollectionResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *collection_resource.mutation_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, collection_resource.mutation_events().count())?;
            (key, sequence_number)
        } else {
            // If the token resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Collection resource not found"
            )));
        };
        let collection_mutation_event =
            CollectionMutationEvent::new(collection_mutation.mutated_field_name().clone());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            COLLECTION_MUTATION_EVENT_TYPE.clone(),
            bcs::to_bytes(&collection_mutation_event)?,
        )?)
    }
}

struct MintTranslator;
impl EventV2Translator for MintTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let mint = Mint::try_from_bytes(v2.event_data())?;
        let fixed_supply_struct_tag = StructTag::from_str("0x4::collection::FixedSupply")?;
        let unlimited_supply_struct_tag = StructTag::from_str("0x4::collection::UnlimitedSupply")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_object_group_resource(
                mint.collection(),
                &fixed_supply_struct_tag,
            )? {
            let fixed_supply_resource: FixedSupplyResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *fixed_supply_resource.mint_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, fixed_supply_resource.mint_events().count())?;
            (key, sequence_number)
        } else if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_object_group_resource(
                mint.collection(),
                &unlimited_supply_struct_tag,
            )?
        {
            let unlimited_supply_resource: UnlimitedSupplyResource =
                bcs::from_bytes(&state_value_bytes)?;
            let key = *unlimited_supply_resource.mint_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, unlimited_supply_resource.mint_events().count())?;
            (key, sequence_number)
        } else {
            // If the collection resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided. The collection may have ConcurrentSupply.
            return Err(AptosDbError::from(anyhow::format_err!(
                "FixedSupply or UnlimitedSupply resource not found"
            )));
        };
        let mint_event = MintEvent::new(mint.index().value, *mint.token());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            MINT_EVENT_TYPE.clone(),
            bcs::to_bytes(&mint_event)?,
        )?)
    }
}

struct BurnTranslator;
impl EventV2Translator for BurnTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let burn = Burn::try_from_bytes(v2.event_data())?;
        let fixed_supply_struct_tag = StructTag::from_str("0x4::collection::FixedSupply")?;
        let unlimited_supply_struct_tag = StructTag::from_str("0x4::collection::UnlimitedSupply")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_object_group_resource(
                burn.collection(),
                &fixed_supply_struct_tag,
            )? {
            let fixed_supply_resource: FixedSupplyResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *fixed_supply_resource.burn_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, fixed_supply_resource.burn_events().count())?;
            (key, sequence_number)
        } else if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_object_group_resource(
                burn.collection(),
                &unlimited_supply_struct_tag,
            )?
        {
            let unlimited_supply_resource: UnlimitedSupplyResource =
                bcs::from_bytes(&state_value_bytes)?;
            let key = *unlimited_supply_resource.burn_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, unlimited_supply_resource.burn_events().count())?;
            (key, sequence_number)
        } else {
            // If the collection resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided. The collection may have ConcurrentSupply.
            return Err(AptosDbError::from(anyhow::format_err!(
                "FixedSupply or UnlimitedSupply resource not found"
            )));
        };
        let burn_event = BurnEvent::new(*burn.index(), *burn.token());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            BURN_EVENT_TYPE.clone(),
            bcs::to_bytes(&burn_event)?,
        )?)
    }
}

struct TokenDepositTranslator;
impl EventV2Translator for TokenDepositTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let deposit = TokenDeposit::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token::TokenStore")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(deposit.account(), &struct_tag)?
        {
            let token_store_resource: TokenStoreResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *token_store_resource.deposit_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, token_store_resource.deposit_events().count())?;
            (key, sequence_number)
        } else {
            // If the token store resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Token store resource not found"
            )));
        };
        let deposit_event = TokenDepositEvent::new(deposit.id().clone(), deposit.amount());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            TOKEN_DEPOSIT_EVENT_TYPE.clone(),
            bcs::to_bytes(&deposit_event)?,
        )?)
    }
}

struct TokenWithdrawTranslator;
impl EventV2Translator for TokenWithdrawTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let withdraw = TokenWithdraw::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token::TokenStore")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(withdraw.account(), &struct_tag)?
        {
            let token_store_resource: TokenStoreResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *token_store_resource.withdraw_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, token_store_resource.withdraw_events().count())?;
            (key, sequence_number)
        } else {
            // If the token store resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Token store resource not found"
            )));
        };
        let withdraw_event = TokenWithdrawEvent::new(withdraw.id().clone(), withdraw.amount());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            TOKEN_WITHDRAW_EVENT_TYPE.clone(),
            bcs::to_bytes(&withdraw_event)?,
        )?)
    }
}

struct BurnTokenTranslator;
impl EventV2Translator for BurnTokenTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let burn = BurnToken::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token::TokenStore")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(burn.account(), &struct_tag)?
        {
            let token_store_resource: TokenStoreResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *token_store_resource.burn_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, token_store_resource.burn_events().count())?;
            (key, sequence_number)
        } else {
            // If the token store resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Token store resource not found"
            )));
        };
        let burn_event = BurnTokenEvent::new(burn.id().clone(), burn.amount());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            BURN_TOKEN_EVENT_TYPE.clone(),
            bcs::to_bytes(&burn_event)?,
        )?)
    }
}

struct MutatePropertyMapTranslator;
impl EventV2Translator for MutatePropertyMapTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let mutate = MutatePropertyMap::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token::TokenStore")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(mutate.account(), &struct_tag)?
        {
            let token_store_resource: TokenStoreResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *token_store_resource.mutate_token_property_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                token_store_resource.mutate_token_property_events().count(),
            )?;
            (key, sequence_number)
        } else {
            // If the token store resource is not found, we skip the event translation to avoid panic
            // because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Token store resource not found"
            )));
        };
        let mutate_event = MutateTokenPropertyMapEvent::new(
            mutate.old_id().clone(),
            mutate.new_id().clone(),
            mutate.keys().clone(),
            mutate.values().clone(),
            mutate.types().clone(),
        );

        Ok(ContractEventV1::new(
            key,
            sequence_number,
            MUTATE_TOKEN_PROPERTY_MAP_EVENT_TYPE.clone(),
            bcs::to_bytes(&mutate_event)?,
        )?)
    }
}

struct MintTokenTranslator;
impl EventV2Translator for MintTokenTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let mint = MintToken::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token::Collections")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(mint.creator(), &struct_tag)?
        {
            let token_store_resource: CollectionsResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *token_store_resource.mint_token_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, token_store_resource.mint_token_events().count())?;
            (key, sequence_number)
        } else {
            // If the collections store resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Collections resource not found"
            )));
        };
        let mint_event = MintTokenEvent::new(mint.id().clone(), mint.amount());

        Ok(ContractEventV1::new(
            key,
            sequence_number,
            MINT_TOKEN_EVENT_TYPE.clone(),
            bcs::to_bytes(&mint_event)?,
        )?)
    }
}

struct CreateCollectionTranslator;
impl EventV2Translator for CreateCollectionTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let create = CreateCollection::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token::Collections")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(create.creator(), &struct_tag)?
        {
            let collections_resource: CollectionsResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *collections_resource.create_collection_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                collections_resource.create_collection_events().count(),
            )?;
            (key, sequence_number)
        } else {
            // If the collections resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Collections resource not found"
            )));
        };
        let create_event = CreateCollectionEvent::new(
            *create.creator(),
            create.collection_name().clone(),
            create.uri().clone(),
            create.description().clone(),
            create.maximum(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            CREATE_COLLECTION_EVENT_TYPE.clone(),
            bcs::to_bytes(&create_event)?,
        )?)
    }
}

struct TokenDataCreationTranslator;
impl EventV2Translator for TokenDataCreationTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let create = TokenDataCreation::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token::Collections")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(create.creator(), &struct_tag)?
        {
            let collections_resource: CollectionsResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *collections_resource.create_token_data_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                collections_resource.create_token_data_events().count(),
            )?;
            (key, sequence_number)
        } else {
            // If the collections resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "Collections resource not found"
            )));
        };
        let create_event = CreateTokenDataEvent::new(
            create.id().clone(),
            create.description().clone(),
            create.maximum(),
            create.uri().clone(),
            *create.royalty_payee_address(),
            create.royalty_points_denominator(),
            create.royalty_points_numerator(),
            create.name().clone(),
            create.mutability_config().clone(),
            create.property_keys().clone(),
            create.property_values().clone(),
            create.property_types().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            CREATE_TOKEN_DATA_EVENT_TYPE.clone(),
            bcs::to_bytes(&create_event)?,
        )?)
    }
}

struct OfferTranslator;
impl EventV2Translator for OfferTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let offer = Offer::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_transfers::PendingClaims")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(offer.account(), &struct_tag)?
        {
            let object_resource: PendingClaimsResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.offer_events().key();
            let sequence_number =
                engine.get_next_sequence_number(&key, object_resource.offer_events().count())?;
            (key, sequence_number)
        } else {
            // If the PendingClaims resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "PendingClaims resource not found"
            )));
        };
        let offer_event = TokenOfferEvent::new(
            *offer.to_address(),
            offer.token_id().clone(),
            offer.amount(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            TOKEN_OFFER_EVENT_TYPE.clone(),
            bcs::to_bytes(&offer_event)?,
        )?)
    }
}

struct CancelOfferTranslator;
impl EventV2Translator for CancelOfferTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let cancel_offer = CancelOffer::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_transfers::PendingClaims")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(cancel_offer.account(), &struct_tag)?
        {
            let object_resource: PendingClaimsResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.cancel_offer_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, object_resource.cancel_offer_events().count())?;
            (key, sequence_number)
        } else {
            // If the PendingClaims resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "PendingClaims resource not found"
            )));
        };
        let cancel_offer_event = TokenCancelOfferEvent::new(
            *cancel_offer.to_address(),
            cancel_offer.token_id().clone(),
            cancel_offer.amount(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            TOKEN_CANCEL_OFFER_EVENT_TYPE.clone(),
            bcs::to_bytes(&cancel_offer_event)?,
        )?)
    }
}

struct ClaimTranslator;
impl EventV2Translator for ClaimTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let claim = Claim::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_transfers::PendingClaims")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(claim.account(), &struct_tag)?
        {
            let object_resource: PendingClaimsResource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.claim_events().key();
            let sequence_number =
                engine.get_next_sequence_number(&key, object_resource.claim_events().count())?;
            (key, sequence_number)
        } else {
            // If the PendingClaims resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "PendingClaims resource not found"
            )));
        };
        let claim_event = TokenClaimEvent::new(
            *claim.to_address(),
            claim.token_id().clone(),
            claim.amount(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            TOKEN_CLAIM_EVENT_TYPE.clone(),
            bcs::to_bytes(&claim_event)?,
        )?)
    }
}

struct CollectionDescriptionMutateTranslator;
impl EventV2Translator for CollectionDescriptionMutateTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let collection_description_mutate =
            CollectionDescriptionMutate::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_resource(
                collection_description_mutate.creator_addr(),
                &struct_tag,
            )? {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.collection_description_mutate_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                object_resource
                    .collection_description_mutate_events()
                    .count(),
            )?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let collection_mutation_event = CollectionDescriptionMutateEvent::new(
            *collection_description_mutate.creator_addr(),
            collection_description_mutate.collection_name().clone(),
            collection_description_mutate.old_description().clone(),
            collection_description_mutate.new_description().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            COLLECTION_DESCRIPTION_MUTATE_EVENT_TYPE.clone(),
            bcs::to_bytes(&collection_mutation_event)?,
        )?)
    }
}

struct CollectionUriMutateTranslator;
impl EventV2Translator for CollectionUriMutateTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let collection_uri_mutate = CollectionUriMutate::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_resource(collection_uri_mutate.creator_addr(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.collection_uri_mutate_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                object_resource.collection_uri_mutate_events().count(),
            )?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let collection_mutation_event = CollectionUriMutateEvent::new(
            *collection_uri_mutate.creator_addr(),
            collection_uri_mutate.collection_name().clone(),
            collection_uri_mutate.old_uri().clone(),
            collection_uri_mutate.new_uri().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            COLLECTION_URI_MUTATE_EVENT_TYPE.clone(),
            bcs::to_bytes(&collection_mutation_event)?,
        )?)
    }
}

struct CollectionMaximumMutateTranslator;
impl EventV2Translator for CollectionMaximumMutateTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let collection_max_mutate = CollectionMaximumMutate::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_resource(collection_max_mutate.creator_addr(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.collection_maximum_mutate_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                object_resource.collection_maximum_mutate_events().count(),
            )?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let collection_mutation_event = CollectionMaximumMutateEvent::new(
            *collection_max_mutate.creator_addr(),
            collection_max_mutate.collection_name().clone(),
            *collection_max_mutate.old_maximum(),
            *collection_max_mutate.new_maximum(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            COLLECTION_MAXIMUM_MUTATE_EVENT_TYPE.clone(),
            bcs::to_bytes(&collection_mutation_event)?,
        )?)
    }
}

struct UriMutationTranslator;
impl EventV2Translator for UriMutationTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let uri_mutation = UriMutation::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(uri_mutation.creator(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.uri_mutate_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, object_resource.uri_mutate_events().count())?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let uri_mutation_event = UriMutationEvent::new(
            *uri_mutation.creator(),
            uri_mutation.collection().clone(),
            uri_mutation.token().clone(),
            uri_mutation.old_uri().clone(),
            uri_mutation.new_uri().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            URI_MUTATION_EVENT_TYPE.clone(),
            bcs::to_bytes(&uri_mutation_event)?,
        )?)
    }
}

struct DefaultPropertyMutateTranslator;
impl EventV2Translator for DefaultPropertyMutateTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let default_property_mutate = DefaultPropertyMutate::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_resource(default_property_mutate.creator(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.default_property_mutate_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                object_resource.default_property_mutate_events().count(),
            )?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let default_property_mutate_event = DefaultPropertyMutateEvent::new(
            *default_property_mutate.creator(),
            default_property_mutate.collection().clone(),
            default_property_mutate.token().clone(),
            default_property_mutate.keys().clone(),
            default_property_mutate.old_values().clone(),
            default_property_mutate.new_values().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            DEFAULT_PROPERTY_MUTATE_EVENT_TYPE.clone(),
            bcs::to_bytes(&default_property_mutate_event)?,
        )?)
    }
}

struct DescriptionMutateTranslator;
impl EventV2Translator for DescriptionMutateTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let description_mutation = DescriptionMutate::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_resource(description_mutation.creator(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.description_mutate_events().key();
            let sequence_number = engine.get_next_sequence_number(
                &key,
                object_resource.description_mutate_events().count(),
            )?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let description_mutation_event = DescriptionMutateEvent::new(
            *description_mutation.creator(),
            description_mutation.collection().clone(),
            description_mutation.token().clone(),
            description_mutation.old_description().clone(),
            description_mutation.new_description().clone(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            DESCRIPTION_MUTATE_EVENT_TYPE.clone(),
            bcs::to_bytes(&description_mutation_event)?,
        )?)
    }
}

struct RoyaltyMutateTranslator;
impl EventV2Translator for RoyaltyMutateTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let royalty_mutation = RoyaltyMutate::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(royalty_mutation.creator(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.royalty_mutate_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, object_resource.royalty_mutate_events().count())?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let royalty_mutation_event = RoyaltyMutateEvent::new(
            *royalty_mutation.creator(),
            royalty_mutation.collection().clone(),
            royalty_mutation.token().clone(),
            *royalty_mutation.old_royalty_numerator(),
            *royalty_mutation.old_royalty_denominator(),
            *royalty_mutation.old_royalty_payee_addr(),
            *royalty_mutation.new_royalty_numerator(),
            *royalty_mutation.new_royalty_denominator(),
            *royalty_mutation.new_royalty_payee_addr(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            ROYALTY_MUTATE_EVENT_TYPE.clone(),
            bcs::to_bytes(&royalty_mutation_event)?,
        )?)
    }
}

struct MaximumMutateTranslator;
impl EventV2Translator for MaximumMutateTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let maximum_mutation = MaximumMutate::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) =
            engine.get_state_value_bytes_for_resource(maximum_mutation.creator(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.maximum_mutate_events().key();
            let sequence_number = engine
                .get_next_sequence_number(&key, object_resource.maximum_mutate_events().count())?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let maximum_mutation_event = MaximumMutateEvent::new(
            *maximum_mutation.creator(),
            maximum_mutation.collection().clone(),
            maximum_mutation.token().clone(),
            *maximum_mutation.old_maximum(),
            *maximum_mutation.new_maximum(),
        );
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            MAXIMUM_MUTATE_EVENT_TYPE.clone(),
            bcs::to_bytes(&maximum_mutation_event)?,
        )?)
    }
}

struct OptInTransferTranslator;
impl EventV2Translator for OptInTransferTranslator {
    fn translate_event_v2_to_v1(
        &self,
        v2: &ContractEventV2,
        engine: &EventV2TranslationEngine,
    ) -> Result<ContractEventV1> {
        let opt_in_transfer = OptInTransfer::try_from_bytes(v2.event_data())?;
        let struct_tag = StructTag::from_str("0x3::token_event_store::TokenEventStoreV1")?;
        let (key, sequence_number) = if let Some(state_value_bytes) = engine
            .get_state_value_bytes_for_resource(opt_in_transfer.account_address(), &struct_tag)?
        {
            let object_resource: TokenEventStoreV1Resource = bcs::from_bytes(&state_value_bytes)?;
            let key = *object_resource.opt_in_events().key();
            let sequence_number =
                engine.get_next_sequence_number(&key, object_resource.opt_in_events().count())?;
            (key, sequence_number)
        } else {
            // If the TokenEventStoreV1 resource is not found, we skip the event translation to
            // avoid panic because the creation number cannot be decided.
            return Err(AptosDbError::from(anyhow::format_err!(
                "TokenEventStoreV1 resource not found"
            )));
        };
        let opt_in_transfer_event = OptInTransferEvent::new(*opt_in_transfer.opt_in());
        Ok(ContractEventV1::new(
            key,
            sequence_number,
            OPT_IN_TRANSFER_EVENT_TYPE.clone(),
            bcs::to_bytes(&opt_in_transfer_event)?,
        )?)
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::non_canonical_partial_ord_impl)]

pub mod inner;
pub mod prefix;
pub mod registry;
#[cfg(test)]
mod tests;

use crate::{
    access_path,
    access_path::AccessPath,
    on_chain_config::OnChainConfig,
    state_store::{
        state_key::{
            inner::{StateKeyDecodeErr, StateKeyTag, TradingNativeKey, TradingNativeKeyTag},
            registry::{Entry, REGISTRY},
        },
        table::TableHandle,
    },
};
use anyhow::Result;
use aptos_crypto::{
    hash::{CryptoHash, DummyHasher},
    HashValue,
};
use bytes::Bytes;
use inner::StateKeyInner;
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag},
    move_resource::MoveResource,
};
use num_traits::FromPrimitive;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp::Ordering,
    fmt,
    fmt::{Debug, Formatter},
    hash::Hash,
    sync::Arc,
};

#[derive(Clone)]
pub struct StateKey(Arc<Entry>);

impl Debug for StateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner().fmt(f)
    }
}

impl StateKey {
    pub fn encoded(&self) -> &Bytes {
        &self.0.encoded
    }

    /// Recovers from serialized bytes in physical storage.
    pub fn decode(val: &[u8]) -> Result<StateKey, StateKeyDecodeErr> {
        use access_path::Path;

        if val.is_empty() {
            return Err(StateKeyDecodeErr::EmptyInput);
        }
        let tag = val[0];
        let state_key_tag =
            StateKeyTag::from_u8(tag).ok_or(StateKeyDecodeErr::UnknownTag { unknown_tag: tag })?;
        let myself = match state_key_tag {
            StateKeyTag::AccessPath => {
                let AccessPath { address, path } = bcs::from_bytes(&val[1..])?;
                let path: Path = bcs::from_bytes(&path)?;
                match path {
                    Path::Code(ModuleId { address, name }) => Self::module(&address, &name),
                    Path::Resource(struct_tag) => Self::resource(&address, &struct_tag)?,
                    Path::ResourceGroup(struct_tag) => Self::resource_group(&address, &struct_tag),
                }
            },
            StateKeyTag::TableItem => {
                const HANDLE_SIZE: usize = std::mem::size_of::<TableHandle>();
                if val.len() < 1 + HANDLE_SIZE {
                    return Err(StateKeyDecodeErr::NotEnoughBytes {
                        tag,
                        num_bytes: val.len(),
                    });
                }
                let handle = bcs::from_bytes(&val[1..1 + HANDLE_SIZE])?;
                Self::table_item(&handle, &val[1 + HANDLE_SIZE..])
            },
            StateKeyTag::Raw => Self::raw(&val[1..]),
            StateKeyTag::TradingNative => {
                // Expected: [tag:1][sub_tag:1][...payload]
                if val.len() < 2 {
                    return Err(StateKeyDecodeErr::NotEnoughBytes {
                        tag,
                        num_bytes: val.len(),
                    });
                }
                let sub_tag = val[1];
                let sub = TradingNativeKeyTag::from_u8(sub_tag).ok_or(
                    StateKeyDecodeErr::UnknownTradingNativeSubTag {
                        unknown_sub_tag: sub_tag,
                    },
                )?;
                match sub {
                    TradingNativeKeyTag::Position => {
                        // [tag:1][sub_tag:1][exchange:32][account:32][market:32]
                        const ADDR: usize = AccountAddress::LENGTH;
                        const POS_LEN: usize = 2 + ADDR * 3;
                        if val.len() != POS_LEN {
                            return Err(StateKeyDecodeErr::NotEnoughBytes {
                                tag,
                                num_bytes: val.len(),
                            });
                        }
                        let exchange = AccountAddress::from_bytes(&val[2..2 + ADDR])
                            .map_err(|e| StateKeyDecodeErr::AnyHow(e.into()))?;
                        let account = AccountAddress::from_bytes(&val[2 + ADDR..2 + ADDR * 2])
                            .map_err(|e| StateKeyDecodeErr::AnyHow(e.into()))?;
                        let market = AccountAddress::from_bytes(&val[2 + ADDR * 2..POS_LEN])
                            .map_err(|e| StateKeyDecodeErr::AnyHow(e.into()))?;
                        Self::position(exchange, account, market)
                    },
                }
            },
        };
        Ok(myself)
    }

    pub fn crypto_hash_ref(&self) -> &HashValue {
        &self.0.hash_value
    }

    pub fn size(&self) -> usize {
        match self.inner() {
            StateKeyInner::AccessPath(access_path) => access_path.size(),
            StateKeyInner::TableItem { handle, key } => handle.size() + key.len(),
            StateKeyInner::Raw(bytes) => bytes.len(),
            StateKeyInner::TradingNative(key) => match key {
                TradingNativeKey::Position { .. } => {
                    // sub_tag (1) + exchange (32) + account (32) + market (32).
                    // Umbrella tag byte is uniform across all StateKey variants
                    // and excluded by convention (see AccessPath/TableItem/Raw
                    // size() impls); the sub-tag is payload-specific to
                    // TradingNative variants and counted here.
                    1 + AccountAddress::LENGTH * 3
                },
            },
        }
    }

    /// This is `pub` only for benchmarking, don't use in production. Use `::resource()`, etc. instead.
    pub fn from_deserialized(deserialized: StateKeyInner) -> Result<Self> {
        use access_path::Path;

        let myself = match deserialized {
            StateKeyInner::AccessPath(AccessPath { address, path }) => {
                match bcs::from_bytes::<Path>(&path) {
                    Err(err) => {
                        if cfg!(feature = "fuzzing") {
                            // note: to make analyze-serde-formats test happy, do not error out
                            //       alternative is to wrap `AccessPath::path: Vec<u8>` in an enum
                            Self::raw(&bcs::to_bytes(&(address, path)).unwrap())
                        } else {
                            return Err(err.into());
                        }
                    },
                    Ok(Path::Code(module_id)) => Self::module_id(&module_id),
                    Ok(Path::Resource(struct_tag)) => Self::resource(&address, &struct_tag)?,
                    Ok(Path::ResourceGroup(struct_tag)) => {
                        Self::resource_group(&address, &struct_tag)
                    },
                }
            },
            StateKeyInner::TableItem { handle, key } => Self::table_item(&handle, &key),
            StateKeyInner::Raw(bytes) => Self::raw(&bytes),
            StateKeyInner::TradingNative(key) => match key {
                TradingNativeKey::Position {
                    exchange,
                    account,
                    market,
                } => Self::position(exchange, account, market),
            },
        };

        Ok(myself)
    }

    /// Construct a persisted Position state key.
    pub fn position(
        exchange: AccountAddress,
        account: AccountAddress,
        market: AccountAddress,
    ) -> Self {
        Self(
            REGISTRY
                .position((exchange, account), &market)
                .get_or_add(&(exchange, account), &market, || {
                    Ok(StateKeyInner::TradingNative(TradingNativeKey::Position {
                        exchange,
                        account,
                        market,
                    }))
                })
                .expect("Position StateKey encode is infallible"),
        )
    }

    pub fn resource(address: &AccountAddress, struct_tag: &StructTag) -> Result<Self> {
        Ok(Self(REGISTRY.resource(struct_tag, address).get_or_add(
            struct_tag,
            address,
            || {
                Ok(StateKeyInner::AccessPath(AccessPath::resource_access_path(
                    *address,
                    struct_tag.clone(),
                )?))
            },
        )?))
    }

    pub fn resource_typed<T: MoveResource>(address: &AccountAddress) -> Result<Self> {
        Self::resource(address, &T::struct_tag())
    }

    pub fn on_chain_config<T: OnChainConfig>() -> Result<Self> {
        Self::resource(T::address(), &T::struct_tag())
    }

    pub fn resource_group(address: &AccountAddress, struct_tag: &StructTag) -> Self {
        Self(
            REGISTRY
                .resource_group(struct_tag, address)
                .get_or_add(struct_tag, address, || {
                    Ok(StateKeyInner::AccessPath(
                        AccessPath::resource_group_access_path(*address, struct_tag.clone()),
                    ))
                })
                .expect("only possible error is resource path serialization"),
        )
    }

    pub fn module(address: &AccountAddress, name: &IdentStr) -> Self {
        Self(
            REGISTRY
                .module(address, name)
                .get_or_add(address, name, || {
                    Ok(StateKeyInner::AccessPath(AccessPath::code_access_path(
                        ModuleId::new(*address, name.to_owned()),
                    )))
                })
                .expect("only possible error is resource path serialization"),
        )
    }

    pub fn module_id(module_id: &ModuleId) -> Self {
        Self::module(&module_id.address, &module_id.name)
    }

    pub fn table_item(handle: &TableHandle, key: &[u8]) -> Self {
        Self(
            REGISTRY
                .table_item(handle, key)
                .get_or_add(handle, key, || {
                    Ok(StateKeyInner::TableItem {
                        handle: *handle,
                        key: key.to_vec(),
                    })
                })
                .expect("only possible error is resource path serialization"),
        )
    }

    pub fn raw(bytes: &[u8]) -> Self {
        Self(
            REGISTRY
                .raw(bytes)
                .get_or_add(bytes, &(), || Ok(StateKeyInner::Raw(bytes.to_vec())))
                .expect("only possible error is resource path serialization"),
        )
    }

    pub fn inner(&self) -> &StateKeyInner {
        &self.0.deserialized
    }

    pub fn get_shard_id(&self) -> usize {
        usize::from(self.crypto_hash_ref().nibble(0))
    }

    pub fn is_aptos_code(&self) -> bool {
        match self.inner() {
            StateKeyInner::AccessPath(access_path) => {
                access_path.is_code()
                    && (access_path.address == AccountAddress::ONE
                        || access_path.address == AccountAddress::THREE
                        || access_path.address == AccountAddress::FOUR)
            },
            _ => false,
        }
    }
}

impl CryptoHash for StateKey {
    type Hasher = DummyHasher;

    fn hash(&self) -> HashValue {
        *self.crypto_hash_ref()
    }
}

impl Serialize for StateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = StateKeyInner::deserialize(deserializer)?;
        Self::from_deserialized(inner).map_err(Error::custom)
    }
}

impl PartialEq for StateKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StateKey {}

impl Hash for StateKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.crypto_hash_ref().as_ref())
    }
}

impl PartialOrd for StateKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // TODO: consider more efficient PartialOrd && Ord, maybe on another wrapper type, so keys
        //       can be hosted more cheaply in a BTreeSet
        self.0.deserialized.partial_cmp(&other.0.deserialized)
    }
}

impl Ord for StateKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.deserialized.cmp(&other.0.deserialized)
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for StateKey {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        use proptest::strategy::Strategy;

        proptest::prelude::any::<StateKeyInner>()
            .prop_map(|inner| StateKey::from_deserialized(inner).unwrap())
            .boxed()
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Flattens a captured read-set into per-resource and per-module maps.
//!
//! On-chain, many resources live inside resource groups (one `StateValue` is a
//! `BTreeMap<StructTag, Bytes>` of group members). MonoMove and the legacy
//! `InMemoryStorage` are both group-unaware, so groups are unpacked into
//! individual `(address, StructTag) -> bytes` entries here. Both VMs then read
//! the same flat per-resource state and their write keys line up.

use anyhow::{Context, Result};
use aptos_types::{
    access_path::Path as AccessPathKind,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_value::StateValue,
    },
};
use bytes::Bytes;
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::StructTag,
};
use std::collections::{BTreeMap, HashMap};

/// A read-set flattened to per-resource and per-module bytes.
#[derive(Default)]
pub struct FlatState {
    /// Resource bytes keyed by publishing address and (instantiated) type.
    pub resources: HashMap<(AccountAddress, StructTag), Bytes>,
    /// Module bytecode keyed by address and module name (framework + user).
    pub modules: HashMap<(AccountAddress, Identifier), Bytes>,
}

impl FlatState {
    /// Builds a `FlatState` from a captured state map, unpacking resource
    /// groups into individual resources. Table-item, raw, and trading-native
    /// keys are skipped (not compared).
    pub fn build(state: &BTreeMap<StateKey, StateValue>) -> Result<Self> {
        let mut flat = FlatState::default();
        for (key, value) in state {
            match key.inner() {
                StateKeyInner::AccessPath(ap) => match ap.get_path() {
                    AccessPathKind::Resource(tag) => {
                        flat.resources
                            .insert((ap.address, tag), value.bytes().clone());
                    },
                    AccessPathKind::ResourceGroup(_) => {
                        let group: BTreeMap<StructTag, Bytes> = bcs::from_bytes(value.bytes())
                            .context("decoding resource group blob")?;
                        for (member_tag, member_bytes) in group {
                            flat.resources
                                .insert((ap.address, member_tag), member_bytes);
                        }
                    },
                    AccessPathKind::Code(module_id) => {
                        flat.modules.insert(
                            (*module_id.address(), module_id.name().to_owned()),
                            value.bytes().clone(),
                        );
                    },
                },
                StateKeyInner::TableItem { .. }
                | StateKeyInner::Raw(_)
                | StateKeyInner::TradingNative(_) => {},
            }
        }
        Ok(flat)
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Overrides applied to a captured state before either VM runs on it.
//!
//! The comparison is only meaningful if both VMs see the same state, so the overrides produce a
//! [`PatchedState`], the only state the V1 and V2 runners accept. A corpus stores the state
//! exactly as read from chain; the overrides run at replay time.

use crate::corpus::{FrameworkSource, Origin};
use anyhow::{Context, Result};
use aptos_infallible::Mutex;
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::{
    account_config::{CoinInfoResource, IntegerResource, OptionalAggregatorV1Resource},
    on_chain_config::Features,
    state_store::{
        state_key::StateKey, state_slot::StateSlot, state_storage_usage::StateStorageUsage,
        state_value::StateValue, StateViewId, StateViewResult, TStateView,
    },
    transaction::Version,
    AptosCoinType,
};
use mono_move_replay_common::gas::make_gas_free;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
};

/// A state every override has been applied to. Only [`PatchedState::new`] creates one.
pub struct PatchedState {
    state: InMemoryStateStore,
    /// The keys the state was observed not to hold.
    absent: BTreeSet<StateKey>,
    report: OverrideReport,
}

/// Which overrides changed the state. An override that changes VM behavior can hide a MonoMove
/// gap, so how often each one fired is reported rather than applied silently.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct OverrideReport {
    /// The overrides that changed the state, by name.
    pub fired: Vec<&'static str>,
}

/// The overrides that depend on where the state came from.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct OverrideConfig {
    /// See [`disable_absent_features`]. Right only when records run against the framework they
    /// ran against on chain: with a newer framework, the VM's defaults for a new chain are the
    /// consistent choice.
    pub disable_absent_features: bool,
}

impl OverrideConfig {
    pub fn for_origin(origin: &Origin) -> Self {
        let on_chain_framework = match origin {
            Origin::Capture => true,
            Origin::LegacyDump { framework, .. } => match framework {
                FrameworkSource::OnChain => true,
                FrameworkSource::Head => false,
            },
        };
        Self {
            disable_absent_features: on_chain_framework,
        }
    }
}

impl PatchedState {
    /// `absent` lists the keys the state was observed not to hold; the overrides that stand in for
    /// an absent value fire only on those, never on a key merely not observed.
    pub fn new(
        state: InMemoryStateStore,
        config: &OverrideConfig,
        absent: &BTreeSet<StateKey>,
    ) -> Result<Self> {
        let mut report = OverrideReport::default();
        if config.disable_absent_features
            && disable_absent_features(&state, absent).context("absent features override")?
        {
            report.fired.push("absent_features");
        }
        // Every run zeroes gas: gas parity is out of scope, and it makes outputs byte-comparable.
        make_gas_free(&state).context("gas override")?;
        report.fired.push("zero_gas");
        if force_integer_supply(&state).context("integer supply override")? {
            report.fired.push("integer_supply");
        }
        Ok(Self {
            state,
            absent: absent.clone(),
            report,
        })
    }

    pub fn report(&self) -> &OverrideReport {
        &self.report
    }
}

impl PatchedState {
    /// A view of this state that records the keys read through it.
    pub fn view(&self) -> ReplayView<'_> {
        ReplayView {
            state: self,
            reads: Mutex::new(BTreeSet::new()),
            on_first_read: None,
        }
    }

    /// [`Self::view`], also calling `on_first_read` with each key the first time it is read, so
    /// that the reads can be reported while the replay runs.
    pub fn view_reporting<'a>(
        &'a self,
        on_first_read: &'a (dyn Fn(&StateKey) + Sync),
    ) -> ReplayView<'a> {
        ReplayView {
            state: self,
            reads: Mutex::new(BTreeSet::new()),
            on_first_read: Some(on_first_read),
        }
    }

    /// The keys among `reads` the state cannot vouch for: absent from it, and not recorded as
    /// absent at capture. A replay that read one ran on a guess, not on chain state.
    pub fn unobserved(&self, reads: &BTreeSet<StateKey>) -> Result<Vec<StateKey>> {
        Ok(self.unobserved_sample(reads, usize::MAX)?.0)
    }

    /// [`Self::unobserved`] for a read set that may be huge: the first `limit` unobserved keys and
    /// how many there are, without collecting the rest.
    pub fn unobserved_sample(
        &self,
        reads: &BTreeSet<StateKey>,
        limit: usize,
    ) -> Result<(Vec<StateKey>, usize)> {
        let mut sample = vec![];
        let mut total: usize = 0;
        for key in reads {
            if !self.absent.contains(key) && self.state.get_state_value(key)?.is_none() {
                total = total.saturating_add(1);
                if sample.len() < limit {
                    sample.push(key.clone());
                }
            }
        }
        Ok((sample, total))
    }
}

/// A [`PatchedState`] that records every key read through it.
pub struct ReplayView<'a> {
    state: &'a PatchedState,
    reads: Mutex<BTreeSet<StateKey>>,
    on_first_read: Option<&'a (dyn Fn(&StateKey) + Sync)>,
}

impl ReplayView<'_> {
    pub fn into_reads(self) -> BTreeSet<StateKey> {
        self.reads.into_inner()
    }

    fn record(&self, key: &StateKey) {
        let first = self.reads.lock().insert(key.clone());
        if let (true, Some(on_first_read)) = (first, self.on_first_read) {
            on_first_read(key);
        }
    }
}

impl TStateView for ReplayView<'_> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.state.id()
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        self.state.get_usage()
    }

    fn next_version(&self) -> Version {
        self.state.next_version()
    }

    fn get_state_slot(&self, key: &StateKey) -> StateViewResult<StateSlot> {
        self.record(key);
        self.state.get_state_slot(key)
    }

    fn get_state_value(&self, key: &StateKey) -> StateViewResult<Option<StateValue>> {
        self.record(key);
        self.state.get_state_value(key)
    }

    fn contains_state_value(&self, key: &StateKey) -> StateViewResult<bool> {
        self.record(key);
        self.state.contains_state_value(key)
    }
}

impl Deref for PatchedState {
    type Target = InMemoryStateStore;

    fn deref(&self) -> &InMemoryStateStore {
        &self.state
    }
}

/// Stores a `Features` config with every flag off if the config was observed absent. Early
/// mainnet predates the config; without it the VM falls back to the defaults for a new chain,
/// which enable code paths (such as the versioned prologue) the framework of that time lacks.
/// Every flag off is how the chain behaved before flags existed. A config not observed at all is
/// left alone, so a replay that reads it is incomplete rather than run on made-up flags.
fn disable_absent_features(
    state: &InMemoryStateStore,
    absent: &BTreeSet<StateKey>,
) -> Result<bool> {
    let key = StateKey::on_chain_config::<Features>()?;
    if !absent.contains(&key) || state.get_state_value(&key)?.is_some() {
        return Ok(false);
    }
    state.set_on_chain_config(&Features { features: vec![] })?;
    Ok(true)
}

/// Rewrites the APT supply from the parallelizable aggregator to a plain integer holding the same
/// value. MonoMove does not implement the aggregator V1 natives, which mainnet reaches only
/// through this supply; the abort codes of the two representations match, so the switch is
/// observable only in the write set, which both VMs see identically.
///
/// Fires only if the state holds the aggregator and its value: a transaction that did not read
/// the value cannot reach the natives.
fn force_integer_supply(state: &InMemoryStateStore) -> Result<bool> {
    let Some(mut coin_info) = state
        .get_resource::<CoinInfoResource<AptosCoinType>>(AccountAddress::ONE)
        .context("undecodable CoinInfo<AptosCoin>")?
    else {
        return Ok(false);
    };
    let Some(OptionalAggregatorV1Resource {
        aggregator: Some(aggregator),
        integer: None,
    }) = coin_info.supply()
    else {
        return Ok(false);
    };
    let Some(value) = state.get_state_value(&aggregator.state_key())? else {
        return Ok(false);
    };
    let value: u128 = bcs::from_bytes(value.bytes()).context("undecodable APT supply")?;
    let limit = aggregator.limit();
    coin_info.set_supply(Some(OptionalAggregatorV1Resource {
        aggregator: None,
        integer: Some(IntegerResource::new(value, limit)),
    }));
    state.set_resource(AccountAddress::ONE, &coin_info)?;
    Ok(true)
}

/// How many times each override fired, over many states.
#[derive(Clone, Debug, Default, Serialize)]
pub struct OverrideCounts(pub BTreeMap<String, usize>);

impl OverrideCounts {
    pub fn add(&mut self, fired: &[String]) {
        for name in fired {
            *self.0.entry(name.clone()).or_default() += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coin_info_key() -> StateKey {
        StateKey::resource_typed::<CoinInfoResource<AptosCoinType>>(&AccountAddress::ONE)
            .expect("key")
    }

    fn state_with_aggregator_supply(value: Option<u128>) -> (InMemoryStateStore, StateKey) {
        let coin_info = CoinInfoResource::<AptosCoinType>::new(
            AccountAddress::from_hex_literal("0xbeef").expect("address"),
            AccountAddress::from_hex_literal("0xcafe").expect("address"),
            1_000,
        );
        let supply_key = coin_info.supply_aggregator_state_key();
        let state = InMemoryStateStore::new_with_state_values(
            [(
                coin_info_key(),
                StateValue::new_legacy(bcs::to_bytes(&coin_info).expect("bcs").into()),
            )]
            .into_iter()
            .chain(value.map(|value| {
                (
                    supply_key.clone(),
                    StateValue::new_legacy(bcs::to_bytes(&value).expect("bcs").into()),
                )
            })),
        );
        (state, supply_key)
    }

    #[test]
    fn aggregator_supply_becomes_an_integer_with_the_same_value() {
        let (state, _) = state_with_aggregator_supply(Some(42));
        assert!(force_integer_supply(&state).expect("override"));
        let value = state
            .get_state_value(&coin_info_key())
            .expect("read")
            .expect("present");
        let coin_info: CoinInfoResource<AptosCoinType> =
            bcs::from_bytes(value.bytes()).expect("decode");
        let supply = coin_info.supply().as_ref().expect("supply");
        assert!(supply.aggregator.is_none());
        assert_eq!(supply.integer.as_ref().map(|i| i.value), Some(42));
        // A second application finds an integer and leaves it alone.
        assert!(!force_integer_supply(&state).expect("override"));
    }

    #[test]
    fn supply_without_its_value_is_left_alone() {
        let (state, _) = state_with_aggregator_supply(None);
        assert!(!force_integer_supply(&state).expect("override"));
    }

    #[test]
    fn only_features_observed_absent_are_all_disabled() {
        let key = StateKey::on_chain_config::<Features>().expect("key");
        let state = InMemoryStateStore::new_with_state_values([]);
        // Not observed: left alone.
        assert!(!disable_absent_features(&state, &BTreeSet::new()).expect("override"));
        assert!(state.get_state_value(&key).expect("read").is_none());
        // Observed absent: every flag off, once.
        let absent = [key].into_iter().collect();
        assert!(disable_absent_features(&state, &absent).expect("override"));
        let features: Features = state.get_on_chain_config().expect("features");
        assert!(!features.is_versioned_transaction_validation_enabled());
        assert!(!disable_absent_features(&state, &absent).expect("override"));
    }

    #[test]
    fn state_without_coin_info_is_left_alone() {
        let state = InMemoryStateStore::new_with_state_values([]);
        assert!(!force_integer_supply(&state).expect("override"));
    }

    #[test]
    fn only_reads_the_corpus_has_no_observation_of_are_unobserved() {
        let present = StateKey::raw(b"present");
        let absent = StateKey::raw(b"absent");
        let unknown = StateKey::raw(b"unknown");
        let state = PatchedState {
            state: InMemoryStateStore::new_with_state_values([(
                present.clone(),
                StateValue::new_legacy(vec![1].into()),
            )]),
            absent: [absent.clone()].into_iter().collect(),
            report: OverrideReport::default(),
        };
        let view = state.view();
        for key in [&present, &absent, &unknown] {
            view.get_state_value(key).expect("read");
        }
        let reads = view.into_reads();
        assert_eq!(reads.len(), 3);
        assert_eq!(state.unobserved(&reads).expect("unobserved"), vec![unknown]);
    }
}

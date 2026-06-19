// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Helpers that extract structured info out of a proposal's `update_sequence`
//! of `ReleaseEntry`s (the entry type is reused from `aptos-release-builder`).

use aptos_release_builder::{components::GasScheduleLocator, ReleaseEntry};

/// The gas snapshots referenced by a release's `Gas` entry, if any.
pub struct GasEntry {
    pub old: Option<GasScheduleLocator>,
    pub new: GasScheduleLocator,
}

/// Find the `Gas` entry, if any.
pub fn find_gas_entry(entries: &[ReleaseEntry]) -> Option<GasEntry> {
    entries.iter().find_map(|entry| {
        if let ReleaseEntry::Gas { old, new } = entry {
            Some(GasEntry {
                old: old.clone(),
                new: new.clone(),
            })
        } else {
            None
        }
    })
}

/// Collect the feature flags enabled / disabled by the `FeatureFlag` entry,
/// formatted as display strings.
pub fn collect_feature_changes(entries: &[ReleaseEntry]) -> (Vec<String>, Vec<String>) {
    let mut enabled = vec![];
    let mut disabled = vec![];
    for entry in entries {
        if let ReleaseEntry::FeatureFlag(features) = entry {
            enabled.extend(features.enabled.iter().map(|f| format!("{:?}", f)));
            disabled.extend(features.disabled.iter().map(|f| format!("{:?}", f)));
        }
    }
    (enabled, disabled)
}

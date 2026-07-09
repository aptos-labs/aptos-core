// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The tool's own config format used for generating a bundle.
//!
//! Supports only one multi-step governance proposal, which is all we need.

use anyhow::{bail, Context, Result};
use aptos_release_builder::{components::ProposalMetadata, ReleaseEntry};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

/// A config for generating a governance bundle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleConfig {
    /// Release / proposal name, e.g. `v1.45.1`; also the bundle's identity.
    pub name: String,
    /// Governance proposal metadata (title, description, URLs).
    pub metadata: ProposalMetadata,
    /// The ordered changes the proposal enacts.
    pub update_sequence: Vec<ReleaseEntry>,
}

impl BundleConfig {
    /// Load and validate a bundle config from a YAML file.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read bundle config {}", path.display()))?;
        let config: BundleConfig = serde_yaml::from_str(&contents)
            .with_context(|| format!("failed to parse bundle config {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    /// Enforce that the proposal touches each on-chain config kind at most once.
    /// Each is a single global resource (the gas schedule, feature flags, the
    /// version, ...), so setting it twice in one proposal is incoherent. Raw
    /// scripts are exempt — they are arbitrary and may legitimately repeat.
    pub fn validate(&self) -> Result<()> {
        let mut seen = BTreeSet::new();
        for entry in &self.update_sequence {
            if let Some(kind) = config_kind(entry)
                && !seen.insert(kind)
            {
                bail!("a governance bundle may set {} at most once", kind);
            }
        }
        Ok(())
    }
}

/// The on-chain config kind an entry sets, or `None` for entries that may
/// legitimately appear more than once (raw scripts).
fn config_kind(entry: &ReleaseEntry) -> Option<&'static str> {
    match entry {
        ReleaseEntry::Framework(_) => Some("the framework"),
        ReleaseEntry::Gas { .. } => Some("the gas schedule"),
        ReleaseEntry::Version(_) => Some("the version"),
        ReleaseEntry::FeatureFlag(_) => Some("feature flags"),
        ReleaseEntry::Consensus(_) => Some("the consensus config"),
        ReleaseEntry::Execution(_) => Some("the execution config"),
        ReleaseEntry::JwkConsensus(_) => Some("the JWK consensus config"),
        ReleaseEntry::Randomness(_) => Some("the randomness config"),
        ReleaseEntry::OidcProviderOps(_) => Some("OIDC providers"),
        ReleaseEntry::RawScript(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed template config (the workflow's default `--release-config`)
    /// must always parse and validate as a `BundleConfig`.
    #[test]
    fn template_config_is_valid() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/framework-release.yaml");
        BundleConfig::load(&path).expect("data/framework-release.yaml must parse and validate");
    }
}

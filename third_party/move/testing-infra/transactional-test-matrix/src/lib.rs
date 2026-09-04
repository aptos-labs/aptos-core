// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared corpus and configuration data for Move transactional test suites.
//!
//! One [`Corpus`] per suite lists its sources and [`MatrixConfig`]s;
//! [`Corpus::resolve`] answers what one (source, config, [`VmBackend`]) cell
//! runs with and where its baseline lives.
//!
//! Sources are identified by their `tests/`-relative path (see [`TESTS_DIR`]);
//! filters, trial names, and baseline paths all use that form.

use move_command_line_common::testing::{add_exp_suffix, EXP_EXT};
use move_model::metadata::LanguageVersion;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod compiler_v2;
mod move_vm;

pub use compiler_v2::{CompilerV2Payload, COMPILER_V2};

/// Directory under a [`Corpus::root`] holding the sources. Source identities
/// are paths relative to the root, so each begins with this directory.
const TESTS_DIR: &str = "tests";
pub use move_vm::{MoveVmPayload, MOVE_VM};

/// VM implementation used to execute a transactional test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmBackend {
    /// Move VM V1 (`move-vm-runtime`), which owns the canonical baselines.
    V1,
    /// Move VM V2 (`mono-move`), compared against those baselines.
    V2,
}

/// Whether a config's trials can run on a given VM backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    Applicable,
    /// May be enabled later, with reason.
    Deferred(&'static str),
    /// Cannot be enabled on this VM backend, with reason.
    NotApplicable(&'static str),
}

/// A directory of test sources together with the configs that run them.
pub struct Corpus<P: 'static> {
    /// V2's override baselines live under a directory of this name, so two
    /// corpora with the same relative source path do not share an override
    /// file.
    pub name: &'static str,
    /// Workspace-relative directory holding the [`TESTS_DIR`] tree.
    pub root: &'static str,
    /// Extensions of files treated as test sources, without the dot.
    pub source_extensions: &'static [&'static str],
    pub configs: &'static [MatrixConfig<P>],
    /// Path fragments identifying sources that need a per-config baseline.
    pub separate_baseline: &'static [&'static str],
    /// Adjusts a config's payload for the VM backend that will run it. The
    /// identity function when this corpus has no backend-dependent settings.
    pub effective_payload: fn(&P, VmBackend) -> P,
}

/// One column of a corpus's matrix.
#[derive(Debug, PartialEq, Eq)]
pub struct MatrixConfig<P: 'static> {
    pub name: &'static str,
    pub experiments: &'static [(&'static str, bool)],
    pub language_version: LanguageVersion,
    /// Source-path fragments to include; an empty list includes all sources.
    pub include: &'static [&'static str],
    /// Source-path fragments to exclude after applying `include`.
    pub exclude: &'static [&'static str],
    /// Corpus-specific settings, interpreted by the consuming suite.
    pub payload: P,
    /// Whether V2 runs this config.
    pub v2: Applicability,
}

impl<P> MatrixConfig<P> {
    /// Whether the include/exclude filters select `identity`.
    pub fn selects(&self, identity: &str) -> bool {
        (self.include.is_empty() || self.include.iter().any(|inc| identity.contains(inc)))
            && !self.exclude.iter().any(|exc| identity.contains(exc))
    }
}

impl<P> Corpus<P> {
    /// Whether `identity` needs a per-config baseline.
    fn needs_separate_baseline(&self, identity: &str) -> bool {
        self.separate_baseline
            .iter()
            .any(|entry| identity.contains(entry))
    }

    /// Resolves one (source, config, VM backend) cell, or [`None`] when the
    /// config's filters do not select the source.
    ///
    /// Selection and applicability are independent: a selected cell may still
    /// be deferred or unsupported by the requested VM backend.
    pub fn resolve<'corpus>(
        &'corpus self,
        config: &'corpus MatrixConfig<P>,
        identity: &str,
        backend: VmBackend,
    ) -> Option<Resolution<'corpus, P>>
    where
        P: Clone,
    {
        if !config.selects(identity) {
            return None;
        }
        Some(Resolution {
            config,
            applicability: match backend {
                VmBackend::V1 => Applicability::Applicable,
                VmBackend::V2 => config.v2,
            },
            effective_payload: (self.effective_payload)(&config.payload, backend),
            canonical_exp_suffix: self
                .needs_separate_baseline(identity)
                .then(|| format!("{}.{}", config.name, EXP_EXT)),
        })
    }

    /// Paths of all source files relative to the corpus root, sorted.
    ///
    /// `corpus_dir` is the directory containing the [`TESTS_DIR`] tree.
    pub fn sources(&self, corpus_dir: &Path) -> Vec<String> {
        let mut identities = WalkDir::new(corpus_dir.join(TESTS_DIR))
            .follow_links(false)
            .min_depth(1)
            .into_iter()
            .flatten()
            .filter(|entry| {
                entry.file_type().is_file()
                    && entry.path().extension().is_some_and(|extension| {
                        self.source_extensions
                            .iter()
                            .any(|candidate| extension == *candidate)
                    })
            })
            .map(|entry| {
                let path = entry
                    .path()
                    .strip_prefix(corpus_dir)
                    .expect("walked path starts at the corpus directory");
                // Identities are `/`-separated so that filters written with `/`
                // match on every platform.
                path.components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect::<Vec<_>>();
        identities.sort_unstable();
        identities
    }
}

/// Resolved information needed to run one selected matrix cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution<'corpus, P: 'static> {
    pub config: &'corpus MatrixConfig<P>,
    /// Applicability for the VM backend passed to `resolve`.
    pub applicability: Applicability,
    /// The config's payload with VM-backend-dependent rules already applied.
    /// Consumers must interpret this rather than [`MatrixConfig::payload`].
    pub effective_payload: P,
    /// Suffix for the canonical baseline: `Some("<config>.exp")` when the
    /// source needs a per-config baseline, else [`None`] for a plain `.exp`.
    pub canonical_exp_suffix: Option<String>,
}

/// Where a V2-owned override baseline for this cell lives:
/// `<root>/<corpus>/<identity without the `tests/` prefix>.<config>.exp`.
///
/// Overrides are always config-qualified, even when the canonical baseline is
/// shared: a divergence under one config need not exist under another.
pub fn v2_override_path<P>(
    v2_override_root: &Path,
    corpus: &Corpus<P>,
    config: &MatrixConfig<P>,
    identity: &str,
) -> PathBuf {
    let relative = identity
        .strip_prefix(&format!("{TESTS_DIR}/"))
        .unwrap_or(identity);
    v2_override_root.join(corpus.name).join(add_exp_suffix(
        Path::new(relative),
        Some(&format!("{}.{}", config.name, EXP_EXT)),
    ))
}

#[cfg(test)]
mod tests;

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! On-disk layout and `bundle.toml` manifest for a governance bundle, and the
//! checks a bundle must pass (see [`verify`]).
//!
//! A governance bundle is a single self-contained directory packaging one
//! on-chain governance proposal, along with additional artifacts for
//! reviews and integrity.
//!
//! ```text
//! bundle-root/
//! ├── bundle.toml            # this manifest (checksums of every other file)
//! ├── config.yaml            # the config used to generate the bundle
//! ├── metadata.json          # the governance proposal's metadata
//! ├── gas/{old,new}.json     # gas schedule snapshots
//! ├── scripts/N-*.move       # the proposal's script sources, for auditing
//! ├── bytecode/N-*.mv        # the compiled scripts actually deployed
//! └── summary/*.md           # human-reviewable change summaries
//! ```

pub mod verify;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Bumped on a breaking change to the bundle layout or manifest schema, so a
/// tool can reject a format it doesn't understand.
pub const BUNDLE_FORMAT_VERSION: u32 = 1;

/// Manifest file name, relative to the bundle root.
pub const BUNDLE_TOML: &str = "bundle.toml";
/// Copy of the config used to generate the bundle, relative to the bundle root.
pub const CONFIG_YAML: &str = "config.yaml";
/// Directory holding gas schedule snapshots.
pub const GAS_DIR: &str = "gas";
/// Directory holding the proposal's governance script sources (audit only).
pub const SCRIPTS_DIR: &str = "scripts";
/// Directory holding the compiled scripts, the artifacts actually deployed.
pub const BYTECODE_DIR: &str = "bytecode";
/// Directory holding human-reviewable summaries.
pub const SUMMARY_DIR: &str = "summary";
/// Governance proposal metadata file name (bundle top level).
pub const METADATA_JSON: &str = "metadata.json";

/// The full `bundle.toml` manifest.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Bundle format version (see [`BUNDLE_FORMAT_VERSION`]).
    pub format_version: u32,
    pub bundle: BundleSection,
    pub source: SourceSection,
    /// A single digest over the bundle's content (see [`IntegritySection`]).
    pub integrity: IntegritySection,
    /// Per-file SHA-256, keyed by bundle-relative path (excludes `bundle.toml`).
    #[serde(default)]
    pub checksums: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleSection {
    pub name: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub commit: String,
    // Deliberately no `tag` field: we record the commit, not a tag derived from
    // the name.
}

/// Integrity data for verifying the bundle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegritySection {
    /// A single content digest over the bundle: recomputing it locally checks
    /// integrity, and matching it against a trusted external anchor (a signed
    /// git tag, the on-chain proposal) establishes provenance.
    pub digest: String,
}

impl BundleManifest {
    /// Build the manifest for the files currently under `bundle_dir`, with the
    /// checksums and digest computed.
    pub fn new(bundle_dir: &Path, bundle: BundleSection, source: SourceSection) -> Result<Self> {
        let mut manifest = Self {
            format_version: BUNDLE_FORMAT_VERSION,
            bundle,
            source,
            integrity: IntegritySection {
                digest: String::new(),
            },
            checksums: compute_checksums(bundle_dir)?,
        };
        manifest.integrity.digest = manifest.compute_digest();
        Ok(manifest)
    }

    /// Serialize and write the manifest to `<bundle_dir>/bundle.toml`.
    pub fn write(&self, bundle_dir: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| anyhow!("failed to serialize bundle.toml: {}", e))?;
        let path = bundle_dir.join(BUNDLE_TOML);
        fs::write(&path, contents)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    /// Compute the bundle's content digest, excluding `created_at`/`branch` so
    /// regenerating from the same source reproduces the same digest.
    pub fn compute_digest(&self) -> String {
        // Hash each field to a fixed-size block, so the concatenation is
        // unambiguous without separators.
        let h = |s: &str| Sha256::digest(s.as_bytes());

        let mut hasher = Sha256::new();
        hasher.update(h(&self.bundle.name));
        hasher.update(h(&self.source.commit));
        for (path, hash) in &self.checksums {
            hasher.update(h(path));
            hasher.update(h(hash));
        }
        hex::encode(hasher.finalize())
    }

    /// Read and parse `<bundle_dir>/bundle.toml`, rejecting a format version the
    /// tool does not understand.
    pub fn read(bundle_dir: &Path) -> Result<Self> {
        let path = bundle_dir.join(BUNDLE_TOML);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let manifest: Self = toml::from_str(&contents)
            .map_err(|e| anyhow!("failed to parse {}: {}", path.display(), e))?;
        if manifest.format_version != BUNDLE_FORMAT_VERSION {
            bail!(
                "unsupported bundle format version {} (this tool supports {})",
                manifest.format_version,
                BUNDLE_FORMAT_VERSION
            );
        }
        Ok(manifest)
    }
}

/// Per-file SHA-256 keyed by bundle-relative path, excluding `bundle.toml`
/// (which can't checksum itself).
pub fn compute_checksums(bundle_dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    visit_files(bundle_dir, bundle_dir, &mut out)?;
    Ok(out)
}

fn visit_files(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    entries.sort();

    for path in entries {
        if path.is_dir() {
            visit_files(&path, root, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| anyhow!("path outside bundle root: {}", e))?
                .to_string_lossy()
                .replace('\\', "/");
            if rel == BUNDLE_TOML {
                continue;
            }
            let bytes =
                fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
            let to_hash = normalize_for_checksum(&rel, &bytes);
            out.insert(rel, hex::encode(Sha256::digest(to_hash.as_ref())));
        }
    }
    Ok(())
}

/// Summary files are hashed with sign-off checkboxes and incidental whitespace
/// normalized away, so ticking a box during review doesn't break the checksum
/// while content changes still do. Other files are hashed verbatim.
fn normalize_for_checksum<'a>(rel: &str, bytes: &'a [u8]) -> Cow<'a, [u8]> {
    let summary_prefix = format!("{}/", SUMMARY_DIR);
    if rel.starts_with(&summary_prefix) {
        let normalized = String::from_utf8_lossy(bytes)
            .replace("[x]", "[ ]")
            .replace("[X]", "[ ]")
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        Cow::Owned(normalized.into_bytes())
    } else {
        Cow::Borrowed(bytes)
    }
}

/// A single difference found while comparing recorded checksums against the
/// files actually present on disk.
#[derive(Debug)]
pub(crate) enum ChecksumError {
    /// File is listed in the manifest but missing on disk.
    Missing(String),
    /// File is present on disk but not listed in the manifest.
    Extra(String),
    /// File exists but its hash differs from the manifest.
    Mismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

impl std::fmt::Display for ChecksumError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChecksumError::Missing(p) => write!(f, "missing file (listed in manifest): {}", p),
            ChecksumError::Extra(p) => write!(f, "unexpected file (not in manifest): {}", p),
            ChecksumError::Mismatch {
                path,
                expected,
                actual,
            } => write!(
                f,
                "checksum mismatch for {}:\n      expected {}\n      actual   {}",
                path, expected, actual
            ),
        }
    }
}

/// Compare the manifest's recorded checksums against the files on disk.
pub(crate) fn verify_checksums(
    bundle_dir: &Path,
    expected: &BTreeMap<String, String>,
) -> Result<Vec<ChecksumError>> {
    let actual = compute_checksums(bundle_dir)?;
    let mut errors = vec![];

    for (path, expected_hash) in expected {
        match actual.get(path) {
            None => errors.push(ChecksumError::Missing(path.clone())),
            Some(actual_hash) if actual_hash != expected_hash => {
                errors.push(ChecksumError::Mismatch {
                    path: path.clone(),
                    expected: expected_hash.clone(),
                    actual: actual_hash.clone(),
                })
            },
            Some(_) => {},
        }
    }
    for path in actual.keys() {
        if !expected.contains_key(path) {
            errors.push(ChecksumError::Extra(path.clone()));
        }
    }

    Ok(errors)
}

/// The bundle's compiled script files, sorted by file name (which is
/// zero-padded, so lexical order is step order).
pub(crate) fn compiled_script_paths(bundle_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(bundle_dir.join(BYTECODE_DIR)) else {
        return vec![];
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "mv").unwrap_or(false))
        .collect();
    paths.sort();
    paths
}

/// The bundle's compiled scripts as `(file stem, bytecode)`, in step order.
pub fn load_compiled_scripts(bundle_dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let paths = compiled_script_paths(bundle_dir);
    if paths.is_empty() {
        bail!(
            "no compiled scripts found in {}",
            bundle_dir.join(BYTECODE_DIR).display()
        );
    }

    paths
        .iter()
        .map(|path| {
            let name = path
                .file_stem()
                .with_context(|| format!("no file stem: {}", path.display()))?
                .to_string_lossy()
                .into_owned();
            let blob =
                fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
            Ok((name, blob))
        })
        .collect()
}

/// Create the bundle directory, refusing to overwrite an existing one.
pub fn create_bundle_dir(bundle_dir: &Path) -> Result<()> {
    if bundle_dir.exists() {
        bail!(
            "bundle directory already exists: {}\n  refusing to overwrite an existing bundle",
            bundle_dir.display()
        );
    }
    fs::create_dir_all(bundle_dir)
        .with_context(|| format!("failed to create {}", bundle_dir.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha3::Sha3_256;

    /// A minimal bundle on disk: one compiled script with its stamped source, a
    /// metadata file, a summary with one unticked box, and a sealed manifest.
    fn write_bundle(dir: &Path) -> BundleManifest {
        for sub_dir in [SCRIPTS_DIR, BYTECODE_DIR, SUMMARY_DIR] {
            fs::create_dir_all(dir.join(sub_dir)).unwrap();
        }
        let blob = b"bytecode";
        fs::write(dir.join(BYTECODE_DIR).join("0-step.mv"), blob).unwrap();
        fs::write(
            dir.join(SCRIPTS_DIR).join("0-step.move"),
            format!(
                "// Script hash: {}\nscript {{ fun main() {{}} }}",
                hex::encode(Sha3_256::digest(blob))
            ),
        )
        .unwrap();
        fs::write(dir.join(METADATA_JSON), b"{}").unwrap();
        fs::write(dir.join(SUMMARY_DIR).join("notes.md"), "- [ ] reviewed\n").unwrap();

        let manifest = BundleManifest::new(
            dir,
            BundleSection {
                name: "test".to_string(),
                created_at: "now".to_string(),
            },
            SourceSection {
                branch: None,
                commit: "abc".to_string(),
            },
        )
        .unwrap();
        manifest.write(dir).unwrap();
        manifest
    }

    #[test]
    fn manifest_round_trips_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let written = write_bundle(dir.path());

        let read = verify::verify(dir.path(), false).unwrap();
        assert_eq!(read.integrity.digest, written.integrity.digest);

        let scripts = load_compiled_scripts(dir.path()).unwrap();
        assert_eq!(scripts, vec![("0-step".to_string(), b"bytecode".to_vec())]);
    }

    #[test]
    fn verification_reports_every_problem() {
        let dir = tempfile::tempdir().unwrap();
        write_bundle(dir.path());

        fs::write(dir.path().join(BYTECODE_DIR).join("0-step.mv"), b"tampered").unwrap();
        fs::write(dir.path().join("extra.txt"), b"extra").unwrap();
        fs::remove_file(dir.path().join(METADATA_JSON)).unwrap();

        let message = verify::verify(dir.path(), false).unwrap_err().to_string();
        for problem in [
            "checksum mismatch for bytecode/0-step.mv",
            "unexpected file (not in manifest): extra.txt",
            "missing file (listed in manifest): metadata.json",
            "missing metadata.json",
            "does not match the hash stamped on its source",
        ] {
            assert!(message.contains(problem), "{}", message);
        }
    }

    #[test]
    fn ticking_a_summary_box_keeps_checksums_stable() {
        let dir = tempfile::tempdir().unwrap();
        write_bundle(dir.path());
        let message = verify::verify(dir.path(), true).unwrap_err().to_string();
        assert!(
            message.contains("notes.md has unchecked boxes"),
            "{}",
            message
        );

        fs::write(
            dir.path().join(SUMMARY_DIR).join("notes.md"),
            "- [x] reviewed\n",
        )
        .unwrap();
        verify::verify(dir.path(), true).unwrap();
        assert_eq!(
            verify::signoff_status(dir.path())
                .into_iter()
                .map(|(_, ticked, total)| (ticked, total))
                .collect::<Vec<_>>(),
            vec![(1, 1)]
        );
    }

    #[test]
    fn unsupported_format_version_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut manifest = write_bundle(dir.path());
        manifest.format_version = BUNDLE_FORMAT_VERSION + 1;
        manifest.write(dir.path()).unwrap();

        let err = BundleManifest::read(dir.path()).unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported bundle format version"));
    }
}

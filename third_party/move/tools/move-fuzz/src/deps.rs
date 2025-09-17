// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{common::Account, language::LanguageSetting, package::FuzzPackage};
use anyhow::{bail, Result};
use aptos_crypto::{ed25519::Ed25519PrivateKey, Uniform};
use log::{debug, info, warn};
use move_core_types::account_address::AccountAddress;
use move_package::{
    resolution::resolution_graph::ResolutionGraph,
    source_package::{
        layout::SourcePackageLayout,
        manifest_parser::parse_move_manifest_from_file,
        parsed_manifest::{SourceManifest, Version},
    },
};
use petgraph::{algo::toposort, graph::DiGraph};
use rand::rngs::OsRng;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    io,
    ops::Deref,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Mark what kind of package this is
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PkgKind {
    /// primary package to be analyzed
    Primary,
    /// a direct or transitive dependency of a primary package
    Dependency,
    /// a dependency that is also part of the Aptos Framework
    Framework,
}

impl PkgKind {
    pub fn is_external_provider_candidate(self) -> bool {
        matches!(self, Self::Primary | Self::Dependency)
    }

    pub fn external_provider_rank(self) -> u8 {
        match self {
            Self::Primary => 0,
            Self::Dependency => 1,
            Self::Framework => 2,
        }
    }
}

/// Mark where the package is sourced from
#[derive(Eq, PartialEq)]
pub enum PkgLocation {
    Local {
        path: PathBuf,
    },
    Remote {
        url: String,
        rev: String,
        subdir: PathBuf,
        download_to: PathBuf,
    },
}

impl PkgLocation {
    pub fn path(&self) -> PathBuf {
        match self {
            Self::Local { path, .. } => path.clone(),
            Self::Remote {
                download_to,
                subdir,
                ..
            } => download_to.join(subdir),
        }
    }
}

impl Display for PkgLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local { path } => write!(f, "fs://{}", path.display()),
            Self::Remote {
                url,
                rev,
                subdir,
                download_to,
            } => write!(
                f,
                "git://{url}:{rev}/{}->{}",
                subdir.display(),
                download_to.display()
            ),
        }
    }
}

/// Mark the version of the package being analyzed
#[derive(Eq, PartialEq, Clone)]
pub struct PkgVersion {
    major: u64,
    minor: u64,
    fix: u64,
}

impl From<Version> for PkgVersion {
    fn from(value: Version) -> Self {
        let (major, minor, fix) = value;
        Self { major, minor, fix }
    }
}

impl Display for PkgVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.fix)
    }
}

/// Named address within a package
#[derive(Copy, Clone)]
pub enum PkgNamedAddr {
    Unset,
    Devel(AccountAddress),
    Fixed(AccountAddress),
}

impl Display for PkgNamedAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unset => write!(f, "_"),
            Self::Devel(addr) => write!(f, "_{}_", addr),
            Self::Fixed(addr) => write!(f, "{}", addr),
        }
    }
}

/// Manifest of the package being analyzed
#[derive(Clone)]
pub struct PkgManifest {
    pub name: String,
    pub path: PathBuf,
    pub version: PkgVersion,
    pub deps: BTreeMap<String, PkgManifest>,
    pub named_addresses: BTreeMap<String, PkgNamedAddr>,
}

/// A wrapper over package manifest that also marks what kind of package this is
pub struct PkgDeclaration {
    pub kind: PkgKind,
    pub manifest: PkgManifest,
}

/// A compiled package prepared for fuzzing, along with its package kind.
pub struct PkgDefinition {
    pub kind: PkgKind,
    pub manifest_path: PathBuf,
    pub package: FuzzPackage,
}

/// A Move audit project composed by a list of packages to audit
pub struct Project {
    pub pkgs: Vec<PkgDeclaration>,
    pub named_accounts: BTreeMap<String, Account>,
    pub language: LanguageSetting,
}

fn alias_group_id_for<'a>(
    address_aliases: &'a BTreeSet<BTreeSet<String>>,
    key: &str,
) -> Result<Option<&'a String>> {
    let mut alias = None;
    for group in address_aliases {
        if !group.contains(key) {
            continue;
        }
        if alias.is_some() {
            bail!("address {key} belongs to two alias groups");
        }
        alias = Some(
            group
                .iter()
                .next()
                .expect("at least one item in an alias group"),
        );
    }
    Ok(alias)
}

fn analyze_package_manifest(
    location: PkgLocation,
    name_opt: Option<String>,
    version: Option<PkgVersion>,
    analyzed_pkgs: &mut BTreeMap<String, PkgManifest>,
    stack: &mut Vec<String>,
    skip_deps_update: bool,
) -> Result<String> {
    // locate and check package root
    let root = location.path();
    if SourcePackageLayout::try_find_root(&root)? != root {
        bail!(
            "invalid package location: {}, manifest file not found",
            root.display()
        );
    }

    // load the manifest
    let manifest = parse_move_manifest_from_file(&root)?;
    let SourceManifest {
        package,
        addresses: _,
        dev_address_assignments: _,
        build: _,
        dependencies,
        dev_dependencies,
    } = manifest;

    // mark the start of analysis
    debug!(
        "{}+ package manifest analysis: {}",
        "  ".repeat(stack.len()),
        package.name,
    );

    // check name match
    let pkg_name = package.name.to_string();
    match name_opt {
        None => (),
        Some(expected_name) => {
            if expected_name != pkg_name {
                bail!(
                    "dependency name mismatch:
                    expect {expected_name}, found {pkg_name}"
                );
            }
        },
    }

    // check version match
    let pkg_version = package.version.into();
    match version {
        None => (),
        Some(expected_version) => {
            if expected_version != pkg_version {
                bail!(
                    "dependency {pkg_name} version mismatch:
                    expect {expected_version}, found {pkg_version}"
                );
            }
        },
    }

    // check if we have analyzed this package
    match analyzed_pkgs.get(&pkg_name) {
        None => (),
        Some(manifest) => {
            // confirm that it is a match
            if root != manifest.path {
                bail!(
                    "location mismatch of base package {pkg_name}: found {}, analyzed {}",
                    root.display(),
                    manifest.path.display(),
                );
            }
            if pkg_version != manifest.version {
                bail!(
                    "version mismatch of base package {pkg_name}: found {pkg_version}, analyzed {}",
                    manifest.version,
                );
            }

            // we have already analyzed this package
            debug!(
                "{}- package manifest analysis: {} (cached)",
                "  ".repeat(stack.len()),
                package.name,
            );
            return Ok(pkg_name);
        },
    }

    // ensure that there are no cyclic dependencies on the package level
    if stack.contains(&pkg_name) {
        bail!("cyclic dependency on package {pkg_name}");
    }
    stack.push(pkg_name);

    // collect named addresses
    let mut named_addresses = BTreeMap::new();
    match manifest.addresses {
        None => (),
        Some(decls) => {
            for (addr_name, addr_config) in decls {
                let addr_val = match addr_config {
                    None => PkgNamedAddr::Unset,
                    Some(a) => PkgNamedAddr::Fixed(a),
                };
                named_addresses.insert(addr_name.to_string(), addr_val);
            }
        },
    }
    match manifest.dev_address_assignments {
        None => (),
        Some(decls) => {
            for (addr_name, addr_val) in decls {
                match named_addresses.get_mut(addr_name.as_str()) {
                    None => bail!(
                        "unrecognized dev assignment for named address '{addr_name}' in package '{}'",
                        package.name
                    ),
                    Some(existing) => match existing {
                        PkgNamedAddr::Unset => {
                            *existing = PkgNamedAddr::Devel(addr_val);
                        },
                        PkgNamedAddr::Devel(_) => unreachable!(
                            "unexpected dev assignment for named address '{addr_name}' in package '{}'",
                            package.name
                        ),
                        PkgNamedAddr::Fixed(fixed_addr) => {
                            // NOTE: it is weird to see a fixed address being re-assigned in the dev-address part.
                            // It might be okay if they are assigned the same value, and it is definitely weird
                            // if they are assigned to different values.
                            if fixed_addr != &addr_val {
                                warn!(
                                    "dev assignment for named address '{addr_name}' is different from \
                                    the fixed assignment in package '{}', this dev-address will be discarded",
                                    package.name
                                );
                            }
                        },
                    },
                }
            }
        },
    }

    // analyze package dependencies
    let mut dep_set = BTreeSet::new();
    for (dep_name, dep_info) in dependencies.into_iter().chain(dev_dependencies) {
        if dep_info.node_info.is_some() {
            bail!("on-chain dependency is not supported yet: {dep_name}");
        }

        // build the information
        let dep_location = match dep_info.git_info.as_ref() {
            None => {
                let dep_path = if dep_info.local.is_absolute() {
                    dep_info.local.clone()
                } else {
                    root.join(&dep_info.local).canonicalize()?
                };
                PkgLocation::Local { path: dep_path }
            },
            Some(git_info) => {
                let dep_path = if git_info.download_to.is_absolute() {
                    git_info.download_to.clone()
                } else {
                    root.join(&git_info.download_to).canonicalize()?
                };
                PkgLocation::Remote {
                    url: git_info.git_url.to_string(),
                    rev: git_info.git_rev.to_string(),
                    subdir: git_info.subdir.clone(),
                    download_to: dep_path,
                }
            },
        };

        // check if we have analyzed this dependency
        let name = dep_name.to_string();
        let optional_version = dep_info.version.as_ref().map(|v| (*v).into());

        match analyzed_pkgs.get(&name) {
            None => (),
            Some(manifest) => {
                // confirm that it is a match
                match optional_version.as_ref() {
                    None => (),
                    Some(v) => {
                        if v != &manifest.version {
                            bail!(
                                "version mismatch of dependency {name}: declared {v}, analyzed {}",
                                manifest.version,
                            );
                        }
                    },
                }
                if dep_location.path() != manifest.path && !is_framework_package(&name) {
                    // HACK: special treatment for Aptos framework packages due
                    // to the mirror repository: https://github.com/aptos-labs/aptos-framework
                    bail!(
                        "location mismatch of dependency {name}: declared {}, analyzed {}",
                        dep_location.path().display(),
                        manifest.path.display()
                    );
                }

                // we have already analyzed this dependency
                if !dep_set.insert(name) {
                    bail!(
                        "dependency {dep_name} is declared more than once in {}",
                        package.name
                    );
                }
                continue;
            },
        }

        // download the dependency first (if it is a remote one)
        if matches!(dep_location, PkgLocation::Remote { .. }) {
            ResolutionGraph::download_and_update_with_lock(
                dep_name,
                &dep_info,
                skip_deps_update,
                &mut io::stdout(),
            )?;
        }

        // recursively analyze the dependency
        let name = analyze_package_manifest(
            dep_location,
            Some(name),
            optional_version,
            analyzed_pkgs,
            stack,
            skip_deps_update,
        )?;
        if !dep_set.insert(name) {
            bail!(
                "dependency {dep_name} is declared more than once in {}",
                package.name
            );
        }
    }

    // mark that we have analyzed this manifest
    let pkg_name = stack
        .pop()
        .unwrap_or_else(|| unreachable!("expect a package on top of stack"));
    assert_eq!(pkg_name, package.name.as_str());

    // duplicate the manifests
    let mut deps = BTreeMap::new();
    for name in dep_set {
        let manifest = analyzed_pkgs.get(&name).expect("manifest");
        deps.insert(name, manifest.clone());
    }

    // construct manifest
    let exists = analyzed_pkgs.insert(pkg_name.clone(), PkgManifest {
        name: pkg_name.clone(),
        path: root,
        version: pkg_version,
        deps,
        named_addresses,
    });
    if exists.is_some() {
        unreachable!("package {} is analyzed twice", package.name);
    }

    // mark the end of the analysis
    debug!(
        "{}- package manifest analysis: {} (new)",
        "  ".repeat(stack.len()),
        package.name,
    );
    Ok(pkg_name)
}

/// Resolve the dependency relation in the whole project
pub fn resolve(
    path: &Path,
    subdirs: BTreeSet<PathBuf>,
    language: LanguageSetting,
    address_aliases: BTreeSet<BTreeSet<String>>,
    resource_mapping: BTreeMap<String, (String, String)>,
    skip_deps_update: bool,
) -> Result<Project> {
    let base = path.canonicalize()?;

    // find move packages within the project directory
    let mut pkgs = vec![];
    for entry in WalkDir::new(base) {
        let entry = entry?;
        let mut entry_path = entry.into_path();
        if entry_path.file_name().expect("filename") == "Move.toml" {
            // obtain package path
            assert!(entry_path.pop());
            let entry_path = entry_path.canonicalize()?;

            // skip if this package is not in the subdir set
            if !subdirs.is_empty() && !subdirs.contains(&entry_path) {
                continue;
            }

            // mark this package as a primary package
            pkgs.push(entry_path);
        }
    }

    // collect packages
    let mut analyzed_pkgs = BTreeMap::new();
    let mut primary_pkgs = BTreeSet::new();
    for path in pkgs {
        let mut stack = vec![];
        let name = analyze_package_manifest(
            PkgLocation::Local { path },
            None,
            None,
            &mut analyzed_pkgs,
            &mut stack,
            skip_deps_update,
        )?;
        assert!(stack.is_empty());
        primary_pkgs.insert(name);
    }
    info!(
        "found {} package(s), out of which {} are primary",
        analyzed_pkgs.len(),
        primary_pkgs.len()
    );

    // consolidate named addresses
    let mut consolidated = BTreeMap::new();
    for pkg in analyzed_pkgs.values() {
        for (addr_name, addr_val) in &pkg.named_addresses {
            match consolidated.get_mut(addr_name) {
                None => {
                    consolidated.insert(addr_name.clone(), *addr_val);
                },
                Some(existing) => match (*existing, *addr_val) {
                    (PkgNamedAddr::Unset, PkgNamedAddr::Unset) => (),
                    (PkgNamedAddr::Unset, PkgNamedAddr::Devel(a)) => {
                        *existing = PkgNamedAddr::Devel(a);
                    },
                    (PkgNamedAddr::Unset, PkgNamedAddr::Fixed(a)) => {
                        *existing = PkgNamedAddr::Fixed(a);
                    },

                    (PkgNamedAddr::Devel(_), PkgNamedAddr::Unset) => (),
                    (PkgNamedAddr::Devel(a1), PkgNamedAddr::Devel(a2)) => {
                        if a1 != a2 {
                            *existing = PkgNamedAddr::Unset;
                        }
                    },
                    (PkgNamedAddr::Devel(_), PkgNamedAddr::Fixed(a)) => {
                        *existing = PkgNamedAddr::Fixed(a);
                    },

                    (PkgNamedAddr::Fixed(a1), PkgNamedAddr::Fixed(a2)) => {
                        if a1 != a2 {
                            bail!("conflicting assignment for named address: {}", addr_name);
                        }
                    },
                    (PkgNamedAddr::Fixed(_), PkgNamedAddr::Devel(_))
                    | (PkgNamedAddr::Fixed(_), PkgNamedAddr::Unset) => (),
                },
            }
        }
    }
    debug!(
        "{} named addresses found and consolidated",
        consolidated.len()
    );

    // check named address used
    for group in &address_aliases {
        for item in group {
            if !consolidated.contains_key(item) {
                bail!("unknown named address in alias group: {item}");
            }
        }
    }
    for (resource, (base, _)) in &resource_mapping {
        if !consolidated.contains_key(resource) {
            bail!("unknown named address in resource mapping: {resource}");
        }
        if !consolidated.contains_key(base) {
            bail!("unknown named address in resource mapping: {base}");
        }
    }

    // unpack the consolidation and assign addresses for address groups
    let mut address_assignments = BTreeMap::new();
    let mut address_alias_group = BTreeMap::new();
    for (key, val) in consolidated {
        let alias = alias_group_id_for(&address_aliases, &key)?.cloned();
        if let Some(group_id) = alias.as_ref() {
            let group = address_aliases
                .iter()
                .find(|group| group.iter().next() == Some(group_id))
                .expect("alias group must exist");

            if group_id == &key {
                let mut resource_assignment = None;
                for member in group {
                    match resource_mapping.get(member) {
                        None => continue,
                        Some((base, seed)) => {
                            if resource_assignment.is_some() {
                                bail!("alias group contains two resource account");
                            }
                            resource_assignment = Some((base.to_string(), seed.to_string()));
                        },
                    }
                }
                let existing = address_assignments.insert(key.clone(), (val, resource_assignment));
                assert!(existing.is_none());
            } else {
                let (group_addr, _) = address_assignments
                    .get_mut(group_id)
                    .expect("alias group already created");

                let should_update = match (group_addr.deref(), val) {
                    (PkgNamedAddr::Fixed(a1), PkgNamedAddr::Fixed(a2)) => {
                        if a1 != &a2 {
                            bail!("conflicting fixed address within an alias group: {key}");
                        }
                        false
                    },
                    (PkgNamedAddr::Fixed(a1), PkgNamedAddr::Devel(a2)) => {
                        if a1 != &a2 {
                            bail!("conflicting fixed and dev address within an alias group: {key}");
                        }
                        false
                    },
                    (PkgNamedAddr::Fixed(_), PkgNamedAddr::Unset) => false,

                    (PkgNamedAddr::Devel(a1), PkgNamedAddr::Fixed(a2)) => {
                        if a1 != &a2 {
                            bail!("conflicting dev and fixed address within an alias group: {key}");
                        }
                        true
                    },
                    (PkgNamedAddr::Devel(a1), PkgNamedAddr::Devel(a2)) => {
                        if a1 != &a2 {
                            bail!("conflicting dev address assignment within an alias group");
                        }
                        false
                    },
                    (PkgNamedAddr::Devel(_), PkgNamedAddr::Unset) => false,

                    (PkgNamedAddr::Unset, PkgNamedAddr::Fixed(_)) => true,
                    (PkgNamedAddr::Unset, PkgNamedAddr::Devel(_)) => true,
                    (PkgNamedAddr::Unset, PkgNamedAddr::Unset) => false,
                };

                if should_update {
                    *group_addr = val;
                }
            }
        }

        if alias.is_none() {
            address_assignments.insert(key.clone(), (val, resource_mapping.get(&key).cloned()));
        }

        // add the alias information
        address_alias_group.insert(key, alias);
    }

    // iteratively build up the account mapping
    let mut named_accounts: BTreeMap<_, Account> = BTreeMap::new();
    loop {
        let mut updated = false;
        let mut pending = false;
        for (key, alias) in &address_alias_group {
            if named_accounts.contains_key(key) {
                continue;
            }

            // we will be updating the accounts for sure
            pending = true;

            // now try to assign an address to this account
            match alias.as_ref() {
                Some(name) if name != key => {
                    let addr = match named_accounts.get(name) {
                        // NOTE: it is possible to have a `None` here when
                        // 1) this alias group is a resource account and
                        // 2) that resource account has not been created yet.
                        None => continue,
                        Some(a) => a.address(),
                    };

                    named_accounts.insert(key.clone(), Account::Ref(addr));
                    updated = true;
                    continue;
                },
                _ => (),
            }

            // either as the first member in the group or as an individual address
            let (named_addr, resource_assignment) = address_assignments.get(key).unwrap();
            let account = match resource_assignment.as_ref() {
                None => match named_addr {
                    PkgNamedAddr::Fixed(addr) => Account::Ref(*addr),
                    PkgNamedAddr::Devel(_) | PkgNamedAddr::Unset => {
                        Account::Owned(Ed25519PrivateKey::generate(&mut OsRng))
                    },
                },
                Some((base, seed)) => {
                    if matches!(named_addr, PkgNamedAddr::Fixed(_) | PkgNamedAddr::Devel(_)) {
                        bail!("a resource account cannot have fixed or devel assignment: {key}");
                    }

                    // check if we have seen the base assignment
                    match named_accounts.get(base) {
                        None => continue,
                        Some(a) => Account::Resource(a.address(), seed.clone()),
                    }
                },
            };

            named_accounts.insert(key.clone(), account);
            updated = true;
        }

        // get out of the loop when there is no pending assignment
        if !pending {
            break;
        }

        // by the end of the loop, if we see pending but no updates, we might
        // run into mutually recursive resource accounts
        if !updated {
            bail!("deadlock in address assignments");
        }
    }

    // additionally check that all aliases are assigned
    for group in address_aliases {
        for name in group {
            if !named_accounts.contains_key(&name) {
                bail!("unused name in address alias declaration: {}", name);
            }
        }
    }

    // build a dependency graph out of these packages
    let mut graph = DiGraph::new();
    let mut index_mapping = BTreeMap::new();
    for name in analyzed_pkgs.keys() {
        let index = graph.add_node(name.clone());
        index_mapping.insert(name.clone(), index);
    }
    for (name, pkg) in &analyzed_pkgs {
        let dst = *index_mapping.get(name).expect("dst node");
        for dep in pkg.deps.keys() {
            let src = *index_mapping.get(dep).expect("src node");
            graph.add_edge(src, dst, ());
        }
    }

    // topologically sort the dependency graph
    let mut pkgs = vec![];
    match toposort(&graph, None) {
        Ok(nodes) => {
            for node in nodes {
                let key = graph.node_weight(node).expect("node");
                let pkg = analyzed_pkgs
                    .remove(key)
                    .unwrap_or_else(|| unreachable!("expect package with name {key}"));
                let is_primary = primary_pkgs.contains(key);
                let is_framework = is_framework_package(key);
                let kind = match (is_primary, is_framework) {
                    (true, true) => bail!("cannot analyze framework package {}", pkg.name),
                    (true, false) => PkgKind::Primary,
                    (false, true) => PkgKind::Framework,
                    (false, false) => PkgKind::Dependency,
                };
                pkgs.push(PkgDeclaration {
                    kind,
                    manifest: pkg,
                });
            }
        },
        Err(cycle) => {
            bail!(
                "unexpected cyclic dependency in packages: {}",
                graph
                    .node_weight(cycle.node_id())
                    .map_or("<unknown>", |e| e.as_str())
            );
        },
    }

    // done
    Ok(Project {
        pkgs,
        named_accounts,
        language,
    })
}

/// Utility: check if the name of a package is Aptos framework package
fn is_framework_package(name: &str) -> bool {
    matches!(
        name,
        "MoveStdlib" | "AptosStdlib" | "AptosFramework" | "AptosToken" | "AptosTokenObjects"
    )
}

#[cfg(test)]
mod tests {
    use super::{alias_group_id_for, PkgKind};
    use std::collections::BTreeSet;

    #[test]
    fn test_alias_group_id_for_finds_group_leader() {
        let aliases = BTreeSet::from([BTreeSet::from(["alpha".to_string(), "beta".to_string()])]);
        let group = alias_group_id_for(&aliases, "beta").unwrap().cloned();
        assert_eq!(group, Some("alpha".to_string()));
    }

    #[test]
    fn test_alias_group_id_for_missing_key() {
        let aliases = BTreeSet::from([BTreeSet::from(["alpha".to_string(), "beta".to_string()])]);
        assert!(alias_group_id_for(&aliases, "gamma").unwrap().is_none());
    }

    #[test]
    fn test_external_provider_policy_allows_deps_but_not_framework() {
        assert!(PkgKind::Primary.is_external_provider_candidate());
        assert!(PkgKind::Dependency.is_external_provider_candidate());
        assert!(!PkgKind::Framework.is_external_provider_candidate());
    }

    #[test]
    fn test_external_provider_rank_prefers_primary_then_dependency() {
        assert!(
            PkgKind::Primary.external_provider_rank()
                < PkgKind::Dependency.external_provider_rank()
        );
        assert!(
            PkgKind::Dependency.external_provider_rank()
                < PkgKind::Framework.external_provider_rank()
        );
    }
}

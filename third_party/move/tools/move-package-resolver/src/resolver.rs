// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    graph::{Dependency, Package, ResolutionGraph},
    identity::{CanonicalGitUrl, CanonicalNodeUrl, PackageIdentity, SourceLocation},
    lock::PackageLock,
    path::{CanonicalPath, NormalizedPath},
};
use anyhow::{anyhow, bail, Result};
use either::Either;
use move_package_cache::{PackageCache, PackageCacheListener};
use move_package_manifest::{self as manifest, PackageLocation, PackageName};
use petgraph::{algo::kosaraju_scc, graph::NodeIndex, visit::EdgeRef};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

// TODOs
// - Addr subst
// - Allow same package name
// - Dep override
// - Fetch transitive deps for on-chain packages
// - Structured errors and error rendering
// - (Low Priority) Symbolic links in git repos
// - (Low Priority) Resolve deps in parallel

/// Checks for cyclic dependencies in the given resolution graph.
fn check_for_cyclic_dependencies(graph: &ResolutionGraph) -> Result<()> {
    let format_scc = |scc: &[NodeIndex]| {
        scc.iter()
            .map(|node| {
                format!(
                    "{} @ {}",
                    graph[*node].identity.name, graph[*node].identity.location
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let sccs = kosaraju_scc(graph)
        .into_iter()
        .filter(|scc| scc.len() > 1)
        .collect::<Vec<_>>();

    if !sccs.is_empty() {
        let sccs = sccs.iter().map(|scc| format_scc(scc)).collect::<Vec<_>>();
        bail!("Cyclic dependencies found:\n{}", sccs.join("\n\n"));
    }

    Ok(())
}

/// Checks if any node has an edge to itself -- this is a special form of cyclic dependency.
fn check_for_self_dependencies(graph: &ResolutionGraph) -> Result<()> {
    let mut result = Vec::new();

    for edge in graph.edge_references() {
        if edge.source() == edge.target() {
            result.push(edge.source());
        }
    }

    result.sort_unstable();
    result.dedup();

    if !result.is_empty() {
        bail!(
            "Found packages with self-dependencies:\n{}",
            result
                .iter()
                .map(|idx| format!(
                    "{} @ {}",
                    graph[*idx].identity.name, graph[*idx].identity.location
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(())
}

/// Checks if two different packages have the same name -- for now we forbid this, but
/// plan to relax it in the future.
fn check_for_name_conflicts(graph: &ResolutionGraph) -> Result<()> {
    let mut name_location_map = BTreeMap::new();

    for node in graph.node_indices() {
        let identity = &graph[node].identity;

        let locations = name_location_map
            .entry(identity.name.as_str())
            .or_insert_with(Vec::new);
        locations.push(&identity.location);
    }

    let conflicts = name_location_map
        .into_iter()
        .filter(|(_name, locations)| locations.len() > 1)
        .map(|(name, locations)| {
            format!(
                "Package name conflict: {}\n{}",
                name,
                locations
                    .iter()
                    .map(|l| format!("  {}", l))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    if !conflicts.is_empty() {
        bail!("{}", conflicts);
    }

    Ok(())
}

/// Resolves all transitive dependencies for the given root package.
/// The results are returned as a [`ResolutionGraph`].
///
/// During resolution, remote dependencies are fetched and cached.
pub async fn resolve(
    package_cache: &PackageCache<impl PackageCacheListener>,
    package_lock: &mut PackageLock,
    root_package_path: impl AsRef<Path>,
    dev_mode: bool,
) -> Result<ResolutionGraph> {
    let mut graph = ResolutionGraph::new();
    let mut resolved = BTreeMap::new();

    let root_package_path = root_package_path.as_ref();

    // TODO: Is there a way to avoid reading the manifest twice?
    let root_package_manifest = move_package_manifest::parse_package_manifest(
        &fs::read_to_string(root_package_path.join("Move.toml"))?,
    )?;

    let root_package_identity = PackageIdentity {
        name: root_package_manifest.package.name.to_string(),
        location: SourceLocation::Local {
            path: CanonicalPath::new(root_package_path)?,
        },
    };

    resolve_package(
        package_cache,
        package_lock,
        &mut graph,
        &mut resolved,
        root_package_identity,
        dev_mode,
    )
    .await?;

    check_for_name_conflicts(&graph)?;
    check_for_self_dependencies(&graph)?;
    check_for_cyclic_dependencies(&graph)?;

    Ok(graph)
}

/// Returns the local path of the given package.
/// - If the package is local, return the path as is.
/// - If the package is remote, fetch it and return its local path within the package cache.
async fn get_package_local_path(
    package_cache: &PackageCache<impl PackageCacheListener>,
    package_lock: &mut PackageLock,
    identity: &PackageIdentity,
) -> Result<PathBuf> {
    Ok(match &identity.location {
        SourceLocation::OnChain {
            node_url,
            package_addr,
        } => {
            let network_version = package_lock.resolve_network_version(node_url).await?;

            package_cache
                .fetch_on_chain_package(node_url, network_version, *package_addr, &identity.name)
                .await?
        },
        SourceLocation::Local { path } => (**path).clone(),
        SourceLocation::Git {
            url,
            commit_id,
            subdir,
        } => {
            let checkout_path = package_cache.checkout_git_repo(url, *commit_id).await?;
            checkout_path.join(subdir)
        },
    })
}

/// Resolves a package identified by the given identity and adds it to the resolution graph.
async fn resolve_package(
    package_cache: &PackageCache<impl PackageCacheListener>,
    package_lock: &mut PackageLock,
    graph: &mut ResolutionGraph,
    resolved: &mut BTreeMap<PackageIdentity, NodeIndex>,
    identity: PackageIdentity,
    dev_mode: bool,
) -> Result<NodeIndex> {
    if let Some(idx) = resolved.get(&identity) {
        return Ok(*idx);
    }

    let local_path = get_package_local_path(package_cache, package_lock, &identity).await?;

    match &identity.location {
        SourceLocation::OnChain { .. } => {
            let node_idx = graph.add_node(Package {
                identity: identity.clone(),
                local_path,
            });
            resolved.insert(identity, node_idx);

            // TODO: fetch transitive deps

            Ok(node_idx)
        },
        SourceLocation::Local { .. } | SourceLocation::Git { .. } => {
            // Read the package manifest
            let manifest_path = local_path.join("Move.toml");
            let contents = fs::read_to_string(&manifest_path).map_err(|err| {
                anyhow!(
                    "failed to read package manifest at {}: {}",
                    manifest_path.display(),
                    err
                )
            })?;
            let package_manifest = move_package_manifest::parse_package_manifest(&contents)?;
            if *package_manifest.package.name != identity.name {
                bail!(
                    "Package name mismatch -- expected {}, got {}",
                    identity.name,
                    package_manifest.package.name
                );
            }

            // Add the package to the graph
            let node_idx = graph.add_node(Package {
                identity: identity.clone(),
                local_path,
            });
            resolved.insert(identity.clone(), node_idx);

            // Resolve all dependencies
            let all_deps = if dev_mode {
                Either::Left(
                    package_manifest
                        .dependencies
                        .into_iter()
                        .chain(package_manifest.dev_dependencies.into_iter()),
                )
            } else {
                Either::Right(package_manifest.dependencies.into_iter())
            };

            for (dep_name, dep) in all_deps {
                let dep_idx = Box::pin(resolve_dependency(
                    package_cache,
                    package_lock,
                    graph,
                    resolved,
                    &identity,
                    &dep_name,
                    dep,
                    dev_mode,
                ))
                .await?;
                graph.add_edge(node_idx, dep_idx, Dependency {});
            }

            Ok(node_idx)
        },
    }
}

/// Resolves a single dependency for a given package.
///
/// Note that in some cases, the child's identity needs to be derived from the parent's identity.
async fn resolve_dependency(
    package_cache: &PackageCache<impl PackageCacheListener>,
    package_lock: &mut PackageLock,
    graph: &mut ResolutionGraph,
    resolved: &mut BTreeMap<PackageIdentity, NodeIndex>,
    parent_identity: &PackageIdentity,
    dep_name: &PackageName,
    dep: manifest::Dependency,
    dev_mode: bool,
) -> Result<NodeIndex> {
    let package_identity = match dep.location {
        PackageLocation::Local { path: local_path } => match &parent_identity.location {
            SourceLocation::Local { path: parent_path } => {
                // Both parent and child are local, so if the child's path is relative,
                // it is relative to the parent's path.
                let dep_manitest_path = if local_path.is_absolute() {
                    local_path
                } else {
                    parent_path.join(local_path)
                };
                let canonical_path = CanonicalPath::new(&dep_manitest_path).map_err(|err| {
                    anyhow!(
                        "failed to find package at {}: {}",
                        dep_manitest_path.display(),
                        err
                    )
                })?;
                PackageIdentity {
                    name: dep_name.to_string(),
                    location: SourceLocation::Local {
                        path: canonical_path,
                    },
                }
            },
            SourceLocation::Git {
                url,
                commit_id,
                subdir,
            } => {
                // Parent is a git dependency while child is local.
                // This makes the child also a git dependency, with path relative to that of the
                // parent's in the same git repo.
                if local_path.is_absolute() {
                    bail!(
                        "local dependency in a git repo cannot be an absolute path: {}",
                        local_path.display()
                    );
                }

                let new_subdir = subdir.join(local_path);
                let normalized_new_subdir = NormalizedPath::new(&new_subdir);
                if let Some(std::path::Component::ParentDir) =
                    normalized_new_subdir.components().next()
                {
                    bail!("subdir outside of repo root: {}", new_subdir.display());
                }

                PackageIdentity {
                    name: dep_name.to_string(),
                    location: SourceLocation::Git {
                        url: url.clone(),
                        commit_id: *commit_id,
                        subdir: normalized_new_subdir,
                    },
                }
            },
            SourceLocation::OnChain { .. } => unreachable!(),
        },
        PackageLocation::Git { url, rev, subdir } => {
            let commit_id = package_lock
                .resolve_git_revision(package_cache, &url, &rev.unwrap())
                .await?;

            let subdir = PathBuf::from_str(&subdir.unwrap_or(String::new()))?;
            if subdir.is_absolute() {
                bail!("subdir cannot be an absolute path: {}", subdir.display());
            }
            let normalized_subdir = NormalizedPath::new(&subdir);
            if let Some(std::path::Component::ParentDir) = normalized_subdir.components().next() {
                bail!("subdir outside of repo root: {}", subdir.display());
            }

            PackageIdentity {
                name: dep_name.to_string(),
                location: SourceLocation::Git {
                    url: CanonicalGitUrl::new(&url)?,
                    commit_id,
                    subdir: normalized_subdir,
                },
            }
        },
        PackageLocation::Aptos {
            node_url,
            package_addr,
        } => {
            let node_url = CanonicalNodeUrl::new(&Url::from_str(&node_url)?)?;

            PackageIdentity {
                name: dep_name.to_string(),
                location: SourceLocation::OnChain {
                    node_url,
                    package_addr,
                },
            }
        },
    };

    resolve_package(
        package_cache,
        package_lock,
        graph,
        resolved,
        package_identity,
        dev_mode,
    )
    .await
}

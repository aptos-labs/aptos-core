// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    package_hooks,
    resolution::digest::compute_digest,
    source_package::{
        layout::SourcePackageLayout,
        manifest_parser::{parse_move_manifest_string, parse_source_manifest},
        parsed_manifest::{
            Dependencies, Dependency, FileName, NamedAddress, PackageDigest, PackageName,
            SourceManifest, SubstOrRename,
        },
    },
    BuildConfig,
};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use move_command_line_common::files::{
    extension_equals, find_filenames, find_move_filenames, FileHash, MOVE_COMPILED_EXTENSION,
};
use move_compiler::command_line::DEFAULT_OUTPUT_DIR;
use move_core_types::account_address::AccountAddress;
use move_symbol_pool::Symbol;
use petgraph::{algo, graphmap::DiGraphMap, Outgoing};
use ptree::{print_tree, TreeBuilder};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
};

pub type ResolvedTable = ResolutionTable<AccountAddress>;
pub type ResolvedPackage = ResolutionPackage<AccountAddress>;
pub type ResolvedGraph = ResolutionGraph<AccountAddress>;

// rename_to => (from_package name, from_address_name)
pub type Renaming = BTreeMap<NamedAddress, (PackageName, NamedAddress)>;
pub type GraphIndex = PackageName;

type ResolutionTable<T> = BTreeMap<NamedAddress, T>;
type ResolvingTable = ResolutionTable<ResolvingNamedAddress>;
type ResolvingGraph = ResolutionGraph<ResolvingNamedAddress>;
type ResolvingPackage = ResolutionPackage<ResolvingNamedAddress>;

#[derive(Debug, Clone)]
pub struct ResolvingNamedAddress {
    value: Rc<RefCell<Option<AccountAddress>>>,
}

/// A `ResolutionGraph` comes in two flavors:
/// 1. a `ResolutionGraph` during resolution (some named addresses may yet be instantiated)
/// 2. a `ResolvedGraph` which is a graph after resolution in which all named addresses have been
///    assigned a value.
///
/// Named addresses can be assigned values in a couple different ways:
/// 1. They can be assigned a value in the declaring package. In this case the value of that
///    named address will always be that value.
/// 2. Can be left unassigned in the declaring package. In this case it can receive its value
///    through unification across the package graph.
///
/// Named addresses can also be renamed in a package and will be re-exported under thes new names in this case.
#[derive(Debug, Clone)]
pub struct ResolutionGraph<T> {
    pub root_package_path: PathBuf,
    /// Build options
    pub build_options: BuildConfig,
    /// Root package
    pub root_package: SourceManifest,
    /// Dependency graph
    pub graph: DiGraphMap<PackageName, ()>,
    /// A mapping of package name to its resolution
    pub package_table: BTreeMap<PackageName, ResolutionPackage<T>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResolutionPackage<T> {
    /// Pointer into the `ResolutionGraph.graph`
    pub resolution_graph_index: GraphIndex,
    /// source manifest for this package
    pub source_package: SourceManifest,
    /// Where this package is located on the filesystem
    pub package_path: PathBuf,
    /// The renaming of addresses performed by this package
    pub renaming: Renaming,
    /// The mapping of addresses for this package (and that are in scope for it)
    pub resolution_table: ResolutionTable<T>,
    /// The digest of the contents of all source files and manifest under the package root
    pub source_digest: PackageDigest,
}

impl ResolvingGraph {
    pub fn new<W: Write>(
        root_package: SourceManifest,
        root_package_path: PathBuf,
        mut build_options: BuildConfig,
        writer: &mut W,
    ) -> Result<ResolvingGraph> {
        if build_options.architecture.is_none() {
            if let Some(info) = &root_package.build {
                build_options.architecture = info.architecture;
            }
        }
        let mut resolution_graph = Self {
            root_package_path: root_package_path.clone(),
            build_options,
            root_package: root_package.clone(),
            graph: DiGraphMap::new(),
            package_table: BTreeMap::new(),
        };

        resolution_graph
            .build_resolution_graph(root_package.clone(), root_package_path, true, writer)
            .with_context(|| {
                format!(
                    "Unable to resolve packages for package '{}'",
                    root_package.package.name
                )
            })?;
        Ok(resolution_graph)
    }

    pub fn resolve(self) -> Result<ResolvedGraph> {
        let ResolvingGraph {
            root_package_path,
            build_options,
            root_package,
            graph,
            package_table,
        } = self;

        let mut unresolved_addresses = Vec::new();

        let resolved_package_table = package_table
            .into_iter()
            .map(|(name, package)| {
                let ResolutionPackage {
                    resolution_graph_index,
                    source_package,
                    package_path,
                    renaming,
                    resolution_table,
                    source_digest,
                } = package;

                let resolved_table = resolution_table
                    .into_iter()
                    .filter_map(|(addr_name, instantiation_opt)| {
                        match *instantiation_opt.value.borrow() {
                            None => {
                                unresolved_addresses.push(format!(
                                    "Named address '{}' in package '{}'",
                                    addr_name, name
                                ));
                                None
                            },
                            Some(addr) => Some((addr_name, addr)),
                        }
                    })
                    .collect::<BTreeMap<_, _>>();
                let resolved_pkg = ResolvedPackage {
                    resolution_graph_index,
                    source_package,
                    package_path,
                    renaming,
                    resolution_table: resolved_table,
                    source_digest,
                };
                (name, resolved_pkg)
            })
            .collect::<BTreeMap<_, _>>();

        if !unresolved_addresses.is_empty() {
            bail!(
                "Unresolved addresses found: [\n{}\n]\n\
                To fix this, add an entry for each unresolved address to the [addresses] section of {}/Move.toml: \
                e.g.,\n[addresses]\nStd = \"0x1\"\n\
                Alternatively, you can also define [dev-addresses] and call with the -d flag",
                unresolved_addresses.join("\n"),
                root_package_path.to_string_lossy()
            )
        }

        Ok(ResolvedGraph {
            root_package_path,
            build_options,
            root_package,
            graph,
            package_table: resolved_package_table,
        })
    }

    fn build_resolution_graph<W: Write>(
        &mut self,
        package: SourceManifest,
        package_path: PathBuf,
        is_root_package: bool,
        writer: &mut W,
    ) -> Result<()> {
        let package_name = package.package.name;
        let package_node_id = match self.package_table.get(&package_name) {
            None => self.get_or_add_node(package_name)?,
            // Same package and we've already resolved it: OK, return early
            Some(other) if other.source_package == package => return Ok(()),
            // Different packages, with same name: Not OK
            Some(other) => {
                bail!(
                    "Conflicting dependencies found: package '{}' conflicts with '{}'",
                    other.source_package.package.name,
                    package.package.name,
                )
            },
        };

        let mut renaming = BTreeMap::new();
        let mut resolution_table = self
            .build_options
            .additional_named_addresses
            .clone()
            .into_iter()
            .map(|(name, addr)| {
                (
                    NamedAddress::from(name),
                    ResolvingNamedAddress::new(Some(addr)),
                )
            })
            .collect();

        // include dev dependencies if in dev mode
        let additional_deps = if self.build_options.dev_mode {
            package.dev_dependencies.clone()
        } else {
            BTreeMap::new()
        };

        for (dep_name, dep) in package
            .dependencies
            .clone()
            .into_iter()
            .chain(additional_deps.into_iter())
        {
            let dep_node_id = self.get_or_add_node(dep_name).with_context(|| {
                format!(
                    "Cycle between packages {} and {} found",
                    package_name, dep_name
                )
            })?;
            self.graph.add_edge(package_node_id, dep_node_id, ());

            let (dep_renaming, dep_resolution_table) = self
                .process_dependency(dep_name, dep, package_path.clone(), writer)
                .with_context(|| {
                    format!(
                        "While resolving dependency '{}' in package '{}'",
                        dep_name, package_name
                    )
                })?;

            ResolutionPackage::extend_renaming(&mut renaming, &dep_name, dep_renaming.clone())
                .with_context(|| {
                    format!(
                        "While resolving address renames in dependency '{}' in package '{}'",
                        dep_name, package_name
                    )
                })?;

            ResolutionPackage::extend_resolution_table(
                &mut resolution_table,
                &dep_name,
                dep_resolution_table,
                dep_renaming,
            )
            .with_context(|| {
                format!(
                    "Resolving named addresses for dependency '{}' in package '{}'",
                    dep_name, package_name
                )
            })?;
        }

        self.unify_addresses_in_package(&package, &mut resolution_table, is_root_package)?;

        let source_digest =
            ResolvingPackage::get_package_digest_for_config(&package_path, &self.build_options)?;

        let resolved_package = ResolutionPackage {
            resolution_graph_index: package_node_id,
            source_package: package,
            package_path,
            renaming,
            resolution_table,
            source_digest,
        };

        self.package_table.insert(package_name, resolved_package);
        Ok(())
    }

    fn unify_addresses_in_package(
        &mut self,
        package: &SourceManifest,
        resolution_table: &mut ResolvingTable,
        is_root_package: bool,
    ) -> Result<()> {
        let package_name = &package.package.name;
        for (name, addr_opt) in package.addresses.clone().unwrap_or_default().into_iter() {
            match resolution_table.get(&name) {
                Some(other) => {
                    other.unify(addr_opt).with_context(|| {
                        format!(
                            "Unable to resolve named address '{}' in \
                             package '{}' when resolving dependencies",
                            name, package_name
                        )
                    })?;
                },
                None => {
                    resolution_table.insert(name, ResolvingNamedAddress::new(addr_opt));
                },
            }
        }

        if self.build_options.dev_mode && is_root_package {
            let mut addr_to_name_mapping = BTreeMap::new();
            for (name, addr) in resolution_table
                .iter()
                .filter(|(_name, addr)| addr.value.borrow().is_some())
            {
                let names = addr_to_name_mapping
                    .entry(addr.value.borrow().unwrap())
                    .or_insert_with(Vec::new);
                names.push(*name);
            }

            for (name, addr) in package
                .dev_address_assignments
                .clone()
                .unwrap_or_default()
                .into_iter()
            {
                match resolution_table.get(&name) {
                    Some(other) => {
                        other.unify(Some(addr)).with_context(|| {
                            format!(
                                "Unable to resolve named address '{}' in\
                                    package '{}' when resolving dependencies in dev mode",
                                name, package_name
                            )
                        })?;
                    },
                    None => {
                        bail!(
                            "Found unbound dev address assignment '{} = 0x{}' in root package '{}'. \
                             Dev addresses cannot introduce new named addresses",
                            name,
                            addr.short_str_lossless(),
                            package_name
                        );
                    },
                }

                if let Some(conflicts) = addr_to_name_mapping.insert(addr, vec![name]) {
                    bail!(
                        "Found non-unique dev address assignment '{name} = 0x{addr}' in root \
                        package '{pkg}'. Dev address assignments must not conflict with any other \
                        assignments in order to ensure that the package will compile with any \
                        possible address assignment. \
                        Assignment conflicts with previous assignments: {conflicts} = 0x{addr}",
                        name = name,
                        addr = addr.short_str_lossless(),
                        pkg = package_name,
                        conflicts = conflicts
                            .into_iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
                }
            }
        }
        Ok(())
    }

    // Process a dependency. `dep_name_in_pkg` is the name assigned to the dependent package `dep`
    // in the source manifest, and we check that this name matches the name of the dependency it is
    // assigned to.
    fn process_dependency<W: Write>(
        &mut self,
        dep_name_in_pkg: PackageName,
        dep: Dependency,
        root_path: PathBuf,
        writer: &mut W,
    ) -> Result<(Renaming, ResolvingTable)> {
        Self::download_and_update_if_remote(
            dep_name_in_pkg,
            &dep,
            self.build_options.skip_fetch_latest_git_deps,
            writer,
        )?;
        let (dep_package, dep_package_dir) =
            Self::parse_package_manifest(&dep, &dep_name_in_pkg, root_path)
                .with_context(|| format!("While processing dependency '{}'", dep_name_in_pkg))?;
        self.build_resolution_graph(dep_package.clone(), dep_package_dir, false, writer)
            .with_context(|| {
                format!("Unable to resolve package dependency '{}'", dep_name_in_pkg)
            })?;

        if dep_name_in_pkg != dep_package.package.name {
            bail!("Name of dependency declared in package '{}' does not match dependency's package name '{}'",
                dep_name_in_pkg,
                dep_package.package.name
            );
        }

        match dep.digest {
            None => (),
            Some(fixed_digest) => {
                let resolved_pkg = self
                    .package_table
                    .get(&dep_name_in_pkg)
                    .context("Unable to find resolved package by name")?;
                if fixed_digest != resolved_pkg.source_digest {
                    bail!(
                        "Source digest mismatch in dependency '{}'. Expected '{}' but got '{}'.",
                        dep_name_in_pkg,
                        fixed_digest,
                        resolved_pkg.source_digest
                    )
                }
            },
        }

        let resolving_dep = &self.package_table[&dep_name_in_pkg];
        let mut renaming = BTreeMap::new();
        let mut resolution_table = resolving_dep.resolution_table.clone();

        // check that address being renamed exists in the dep that is being renamed/imported
        if let Some(dep_subst) = dep.subst {
            for (name, rename_from_or_assign) in dep_subst.into_iter() {
                match rename_from_or_assign {
                    SubstOrRename::RenameFrom(ident) => {
                        // Make sure dep has the address that we're importing
                        if !resolving_dep.resolution_table.contains_key(&ident) {
                            bail!(
                                "Tried to rename named address {0} from package '{1}'.\
                                However, {1} does not contain that address",
                                ident,
                                dep_name_in_pkg
                            );
                        }

                        // Apply the substitution, NB that the refcell for the address's value is kept!
                        if let Some(other_val) = resolution_table.remove(&ident) {
                            resolution_table.insert(name, other_val);
                        }

                        if renaming.insert(name, (dep_name_in_pkg, ident)).is_some() {
                            bail!("Duplicate renaming of named address '{0}' found for dependency {1}",
                                name,
                                dep_name_in_pkg,
                            );
                        }
                    },
                    SubstOrRename::Assign(value) => {
                        resolution_table
                            .get(&name)
                            .map(|named_addr| named_addr.unify(Some(value)))
                            .transpose()
                            .with_context(|| {
                                format!(
                                    "Unable to assign value to named address {} in dependency {}",
                                    name, dep_name_in_pkg
                                )
                            })?;
                    },
                }
            }
        }

        Ok((renaming, resolution_table))
    }

    fn get_or_add_node(&mut self, package_name: PackageName) -> Result<GraphIndex> {
        if self.graph.contains_node(package_name) {
            // If we encounter a node that we've already added we should check for cycles
            if algo::is_cyclic_directed(&self.graph) {
                // get the first cycle. Exists because we found a cycle above.
                let mut cycle = algo::kosaraju_scc(&self.graph)[0]
                    .iter()
                    .map(|node| node.as_str().to_string())
                    .collect::<Vec<_>>();
                // Add offending node at end to complete the cycle for display
                cycle.push(package_name.as_str().to_string());
                bail!("Found cycle between packages: {}", cycle.join(" -> "));
            }
            Ok(package_name)
        } else {
            Ok(self.graph.add_node(package_name))
        }
    }

    fn parse_package_manifest(
        dep: &Dependency,
        dep_name: &PackageName,
        mut root_path: PathBuf,
    ) -> Result<(SourceManifest, PathBuf)> {
        root_path.push(&dep.local);
        match fs::read_to_string(root_path.join(SourcePackageLayout::Manifest.path())) {
            Ok(contents) => {
                let source_package: SourceManifest =
                    parse_move_manifest_string(contents).and_then(parse_source_manifest)?;
                Ok((source_package, root_path))
            },
            Err(_) => Err(anyhow::format_err!(
                "Unable to find package manifest for '{}' at {:?}",
                dep_name,
                SourcePackageLayout::Manifest.path().join(root_path),
            )),
        }
    }

    pub fn download_dependency_repos<W: Write>(
        manifest: &SourceManifest,
        build_options: &BuildConfig,
        root_path: &Path,
        writer: &mut W,
    ) -> Result<()> {
        // include dev dependencies if in dev mode
        let empty_deps;
        let additional_deps = if build_options.dev_mode {
            &manifest.dev_dependencies
        } else {
            empty_deps = Dependencies::new();
            &empty_deps
        };

        for (dep_name, dep) in manifest.dependencies.iter().chain(additional_deps.iter()) {
            Self::download_and_update_if_remote(
                *dep_name,
                dep,
                build_options.skip_fetch_latest_git_deps,
                writer,
            )?;

            let (dep_manifest, _) =
                Self::parse_package_manifest(dep, dep_name, root_path.to_path_buf())
                    .with_context(|| format!("While processing dependency '{}'", *dep_name))?;
            // download dependencies of dependencies
            Self::download_dependency_repos(&dep_manifest, build_options, root_path, writer)?;
        }
        Ok(())
    }

    fn download_and_update_if_remote<W: Write>(
        dep_name: PackageName,
        dep: &Dependency,
        skip_fetch_latest_git_deps: bool,
        writer: &mut W,
    ) -> Result<()> {
        if let Some(git_info) = &dep.git_info {
            let git_url = git_info.git_url.as_str();
            let git_rev = git_info.git_rev.as_str();
            let git_path = &git_info.download_to.display().to_string();

            // If there is no cached dependency, download it
            if !git_info.download_to.exists() {
                writeln!(
                    writer,
                    "{} {}",
                    "FETCHING GIT DEPENDENCY".bold().green(),
                    git_url,
                )?;

                // If the cached folder does not exist, download and clone accordingly
                Command::new("git")
                    .args(["clone", git_url, git_path])
                    .output()
                    .map_err(|_| {
                        anyhow::anyhow!("Failed to clone Git repository for package '{}'", dep_name)
                    })?;
                Command::new("git")
                    .args(["-C", git_path, "checkout", git_rev])
                    .output()
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "Failed to checkout Git reference '{}' for package '{}'",
                            git_rev,
                            dep_name
                        )
                    })?;
            } else if !skip_fetch_latest_git_deps {
                // Update the git dependency
                // Check first that it isn't a git rev (if it doesn't work, just continue with the fetch)
                if let Ok(rev) = Command::new("git")
                    .args(["-C", git_path, "rev-parse", "--verify", git_rev])
                    .output()
                {
                    if let Ok(parsable_version) = String::from_utf8(rev.stdout) {
                        // If it's exactly the same, then it's a git rev
                        if parsable_version.trim().starts_with(git_rev) {
                            return Ok(());
                        }
                    }
                }

                let tag = Command::new("git")
                    .args(["-C", git_path, "tag", "--list", git_rev])
                    .output();

                if let Ok(tag) = tag {
                    if let Ok(parsable_version) = String::from_utf8(tag.stdout) {
                        // If it's exactly the same, then it's a git tag, for now tags won't be updated
                        // Tags don't easily update locally and you can't use reset --hard to cleanup
                        // any extra files
                        if parsable_version.trim().starts_with(git_rev) {
                            return Ok(());
                        }
                    }
                }

                writeln!(
                    writer,
                    "{} {}",
                    "UPDATING GIT DEPENDENCY".bold().green(),
                    git_url,
                )?;
                // If the current folder exists, do a fetch and reset to ensure that the branch
                // is up to date
                // NOTE: this means that you must run the package system with a working network connection
                let status = Command::new("git")
                    .args([
                        "-C",
                        git_path,
                        "fetch",
                        "origin",
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "Failed to fetch latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps",
                            dep_name
                        )
                    })?;

                if !status.success() {
                    return Err(anyhow::anyhow!(
                            "Failed to fetch to latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
                            dep_name,
                        status
                        ));
                }
                let status = Command::new("git")
                    .args([
                        "-C",
                        git_path,
                        "reset",
                        "--hard",
                        &format!("origin/{}", git_rev)
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps",
                            git_rev,
                            dep_name
                        )
                    })?;
                if !status.success() {
                    return Err(anyhow::anyhow!(
                            "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
                            git_rev,
                            dep_name,
                        status
                        ));
                }
            }
        }
        if let Some(node_info) = &dep.node_info {
            package_hooks::resolve_custom_dependency(dep_name, node_info)?
        }
        Ok(())
    }
}

impl ResolvingPackage {
    // Extend and check for duplicate names in rename_to
    fn extend_renaming(
        renaming: &mut Renaming,
        dep_name: &PackageName,
        dep_renaming: Renaming,
    ) -> Result<()> {
        for (rename_to, rename_from) in dep_renaming.into_iter() {
            // We cannot rename multiple named addresses to the same name. In the future we'll want
            // to support this.
            if renaming.insert(rename_to, rename_from).is_some() {
                bail!(
                    "Duplicate renaming of named address '{}' found in dependency '{}'",
                    rename_to,
                    dep_name
                );
            }
        }
        Ok(())
    }

    // The resolution table contains the transitive closure of addresses that are known in that
    // package. Extends the package's resolution table and checks for duplicate renamings that
    // conflict during this process.
    fn extend_resolution_table(
        resolution_table: &mut ResolvingTable,
        dep_name: &PackageName,
        dep_resolution_table: ResolvingTable,
        dep_renaming: Renaming,
    ) -> Result<()> {
        let renames = dep_renaming
            .into_iter()
            .map(|(rename_to, (_, rename_from))| (rename_from, rename_to))
            .collect::<BTreeMap<_, _>>();

        for (addr_name, addr_value) in dep_resolution_table.into_iter() {
            let addr_name = renames.get(&addr_name).cloned().unwrap_or(addr_name);
            if let Some(other) = resolution_table.insert(addr_name, addr_value.clone()) {
                // They need to be the same refcell so resolve to the same location if there are any
                // possible reassignments
                if other.value != addr_value.value {
                    bail!(
                        "Named address '{}' in dependency '{}' is already set to '{}' but was then reassigned to '{}'",
                        &addr_name,
                        dep_name,
                        match other.value.take() {
                            None => "unassigned".to_string(),
                            Some(addr) => format!("0x{}", addr.short_str_lossless()),
                        },
                        match addr_value.value.take() {
                            None => "unassigned".to_string(),
                            Some(addr) => format!("0x{}", addr.short_str_lossless()),
                        }
                    );
                }
            }
        }

        Ok(())
    }

    fn get_source_paths_for_config(
        package_path: &Path,
        config: &BuildConfig,
    ) -> Result<Vec<PathBuf>> {
        let mut places_to_look = Vec::new();
        let mut add_path = |layout_path: SourcePackageLayout| {
            let path = package_path.join(layout_path.path());
            if layout_path.is_optional() && !path.exists() {
                return;
            }
            places_to_look.push(path)
        };

        add_path(SourcePackageLayout::Sources);
        add_path(SourcePackageLayout::Scripts);

        if config.dev_mode {
            add_path(SourcePackageLayout::Examples);
            add_path(SourcePackageLayout::Tests);
        }
        Ok(places_to_look)
    }

    fn get_build_paths(package_path: &Path) -> Result<Vec<PathBuf>> {
        let mut places_to_look = Vec::new();
        let path = package_path.join(Path::new(DEFAULT_OUTPUT_DIR));
        if path.exists() {
            places_to_look.push(path);
        }
        Ok(places_to_look)
    }

    fn get_package_digest_for_config(
        package_path: &Path,
        config: &BuildConfig,
    ) -> Result<PackageDigest> {
        let mut source_paths = Self::get_source_paths_for_config(package_path, config)?;
        source_paths.push(package_path.join(SourcePackageLayout::Manifest.path()));
        compute_digest(source_paths.as_slice())
    }
}

impl ResolvingNamedAddress {
    pub fn new(address_opt: Option<AccountAddress>) -> Self {
        Self {
            value: Rc::new(RefCell::new(address_opt)),
        }
    }

    pub fn unify(&self, address_opt: Option<AccountAddress>) -> Result<()> {
        match address_opt {
            None => Ok(()),
            Some(addr_val) => match &mut *self.value.borrow_mut() {
                Some(current_value) if current_value != &addr_val =>
                    bail!("Attempted to assign a different value '0x{}' to an a already-assigned named address '0x{}'",
                        addr_val.short_str_lossless(), current_value.short_str_lossless()
                    ),
                Some(_) => Ok(()),
                x @ None => {
                    *x = Some(addr_val);
                    Ok(())
                }
            },
        }
    }
}

impl ResolvedGraph {
    pub fn get_package(&self, package_ident: &PackageName) -> &ResolvedPackage {
        self.package_table.get(package_ident).unwrap()
    }

    fn print_info_dfs(&self, current_node: &PackageName, tree: &mut TreeBuilder) -> Result<()> {
        let pkg = self.package_table.get(current_node).unwrap();

        for (name, addr) in &pkg.resolution_table {
            tree.add_empty_child(format!("{}:0x{}", name, addr.short_str_lossless()));
        }

        for node in self.graph.neighbors_directed(*current_node, Outgoing) {
            tree.begin_child(node.to_string());
            self.print_info_dfs(&node, tree)?;
            tree.end_child();
        }
        Ok(())
    }

    pub fn print_info(&self) -> Result<()> {
        let root = self.root_package.package.name;
        let mut tree = TreeBuilder::new(root.to_string());
        self.print_info_dfs(&root, &mut tree)?;
        let tree = tree.build();
        print_tree(&tree)?;
        Ok(())
    }

    pub fn extract_named_address_mapping(
        &self,
    ) -> impl Iterator<Item = (Symbol, AccountAddress)> + '_ {
        let rooot_package_name = &self.root_package.package.name;
        let root_package = self
            .package_table
            .get(rooot_package_name)
            .expect("Failed to find root package in package table -- this should never happen");

        root_package
            .resolution_table
            .iter()
            .map(|(name, addr)| (*name, *addr))
    }

    pub fn file_sources(&self) -> BTreeMap<FileHash, (Symbol, String)> {
        self.package_table
            .iter()
            .flat_map(|(_, rpkg)| {
                rpkg.get_sources(&self.build_options)
                    .unwrap()
                    .iter()
                    .map(|fname| {
                        let contents = fs::read_to_string(Path::new(fname.as_str())).unwrap();
                        let fhash = FileHash::new(&contents);
                        (fhash, (*fname, contents))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .collect()
    }

    pub fn contains_renaming(&self) -> Option<PackageName> {
        // Make sure no renamings have been performed
        for (pkg_name, pkg) in self.package_table.iter() {
            if !pkg.renaming.is_empty() {
                return Some(*pkg_name);
            }
        }
        None
    }
}

impl ResolvedPackage {
    pub fn get_sources(&self, config: &BuildConfig) -> Result<Vec<FileName>> {
        let places_to_look =
            ResolvingPackage::get_source_paths_for_config(&self.package_path, config)?
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>();
        Ok(find_move_filenames(&places_to_look, false)?
            .into_iter()
            .map(Symbol::from)
            .collect())
    }

    pub fn get_bytecodes(&self) -> Result<Vec<FileName>> {
        let path = ResolvingPackage::get_build_paths(&self.package_path)?;
        let places_to_look = path
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        Ok(find_filenames(&places_to_look, |path| {
            extension_equals(path, MOVE_COMPILED_EXTENSION)
        })?
        .into_iter()
        .map(Symbol::from)
        .collect())
    }

    /// Returns the transitive dependencies of this package in dependency order
    #[allow(clippy::needless_collect)]
    pub fn transitive_dependencies(&self, resolved_graph: &ResolvedGraph) -> BTreeSet<PackageName> {
        let mut seen = BTreeSet::new();
        let resolve_package = |package_name: PackageName| {
            let mut package_deps = resolved_graph
                .package_table
                .get(&package_name)
                .unwrap()
                .transitive_dependencies(resolved_graph);
            package_deps.insert(package_name);
            package_deps
        };

        let immediate_deps = self.immediate_dependencies(resolved_graph);
        let transitive_deps: Vec<_> = immediate_deps
            .into_iter()
            .flat_map(resolve_package)
            .collect();

        transitive_deps
            .into_iter()
            .filter(|ident| {
                if !seen.contains(ident) {
                    seen.insert(*ident);
                    true
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn immediate_dependencies(&self, resolved_graph: &ResolvedGraph) -> BTreeSet<PackageName> {
        if resolved_graph.build_options.dev_mode {
            self.source_package
                .dependencies
                .keys()
                .chain(self.source_package.dev_dependencies.keys())
                .copied()
                .collect()
        } else {
            self.source_package.dependencies.keys().copied().collect()
        }
    }
}

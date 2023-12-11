use std::collections::{ BTreeMap, BTreeSet };
use anyhow::Result;
use move_symbol_pool::Symbol;
use petgraph::algo::toposort;
use move_package::{
    resolution::resolution_graph::{ ResolvedPackage, ResolvedTable, ResolvedGraph, Renaming },

    source_package::parsed_manifest::PackageName,
};
use move_compiler::shared::{ NamedAddressMap, NumericalAddress, PackagePaths };

pub trait TraitResolvedGraph {
    /// Check all packages for cyclic dependencies
    fn check_cyclic_dependency(&self) -> Result<Vec<PackageName>>;
    /// root_package
    fn get_root_package(&self) -> &ResolvedPackage;
    /// root_package and all the source file paths of all its dependent packages
    fn get_root_package_paths(&self) -> Result<(/* sources */ PackagePaths, /* deps */ Vec<PackagePaths>)>;
    /// All dependency packages for root_package
    fn get_root_package_transitive_dependencies(&self) -> BTreeSet<PackageName>;
    /// Direct dependency package of root_package
    fn get_root_package_immediate_dependencies(&self) -> BTreeSet<PackageName>;
}

impl TraitResolvedGraph for ResolvedGraph {
    fn check_cyclic_dependency(&self) -> Result<Vec<PackageName>> {
        let mut sorted_deps = match toposort(&self.graph, None) {
            Ok(nodes) => nodes,
            Err(err) => {
                // Is a DAG after resolution otherwise an error should be raised from that.
                anyhow::bail!("IPE: Cyclic dependency found after resolution {:?}", err)
            }
        };
        sorted_deps.reverse();
        return Ok(sorted_deps);
    }

    fn get_root_package(&self) -> &ResolvedPackage {
        &self.package_table[&self.root_package.package.name]
    }

    fn get_root_package_paths(&self) -> Result<(/* sources */ PackagePaths, /* deps */ Vec<PackagePaths>)> {
        let root_package = self.get_root_package();

        let transitive_dependencies = self
            .get_root_package_transitive_dependencies()
            .into_iter()
            .map(|package_name| {
                let dep_package = self.package_table.get(&package_name).unwrap();
                let dep_source_paths = dep_package.get_sources(&self.build_options).unwrap();
                // (name, source paths, address mapping)
                (package_name, dep_source_paths, &dep_package.resolution_table)
            })
            .collect();

        make_source_and_deps_for_compiler(&self, root_package, transitive_dependencies)
    }

    fn get_root_package_transitive_dependencies(&self) -> BTreeSet<PackageName> {
        self.get_root_package().transitive_dependencies(&self)
    }

    fn get_root_package_immediate_dependencies(&self) -> BTreeSet<PackageName> {
        self.get_root_package().immediate_dependencies(&self)
    }
}

fn named_address_mapping_for_compiler(resolution_table: &ResolvedTable) -> BTreeMap<Symbol, NumericalAddress> {
    resolution_table
        .iter()
        .map(|(ident, addr)| {
            let parsed_addr = NumericalAddress::new(addr.into_bytes(), move_compiler::shared::NumberFormat::Hex);
            (*ident, parsed_addr)
        })
        .collect::<BTreeMap<_, _>>()
}

fn apply_named_address_renaming(
    current_package_name: Symbol,
    address_resolution: BTreeMap<Symbol, NumericalAddress>,
    renaming: &Renaming
) -> NamedAddressMap {
    let package_renamings = renaming
        .iter()
        .filter_map(|(rename_to, (package_name, from_name))| {
            if package_name == &current_package_name { Some((from_name, *rename_to)) } else { None }
        })
        .collect::<BTreeMap<_, _>>();

    address_resolution
        .into_iter()
        .map(|(name, value)| {
            let new_name = package_renamings.get(&name).copied();
            (new_name.unwrap_or(name), value)
        })
        .collect()
}

fn make_source_and_deps_for_compiler(
    resolution_graph: &ResolvedGraph,
    root: &ResolvedPackage,
    deps: Vec<(/* name */ Symbol, /* source paths */ Vec<Symbol>, /* address mapping */ &ResolvedTable)>
) -> Result<(/* sources */ PackagePaths, /* deps */ Vec<PackagePaths>)> {
    let deps_package_paths = deps
        .into_iter()
        .map(|(name, source_paths, resolved_table)| {
            let paths = source_paths.into_iter().collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
            let named_address_map = named_address_mapping_for_compiler(resolved_table);
            Ok(PackagePaths {
                name: Some(name),
                paths,
                named_address_map,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let root_named_addrs = apply_named_address_renaming(
        root.source_package.package.name,
        named_address_mapping_for_compiler(&root.resolution_table),
        &root.renaming
    );
    let sources = root.get_sources(&resolution_graph.build_options)?;
    let source_package_paths = PackagePaths {
        name: Some(root.source_package.package.name),
        paths: sources,
        named_address_map: root_named_addrs,
    };
    Ok((source_package_paths, deps_package_paths))
}

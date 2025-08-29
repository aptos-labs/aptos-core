// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::identity::PackageIdentity;
use petgraph::{visit::EdgeRef, Graph};
use std::{
    collections::BTreeMap,
    fmt::Write,
    path::{Path, PathBuf},
};

/// Represents a node in the resolution graph, with metadata attached.
#[derive(Debug)]
pub struct Package {
    pub identity: PackageIdentity,
    pub local_path: PathBuf,
}

/// Represents an edge in the resolution graph -- a dependency between two packages.
#[derive(Debug)]
pub struct Dependency {}

pub type ResolutionGraph = Graph<Package, Dependency>;

/// Converts a [`ResolutionGraph`] into a Mermaid flowchart for visualization.
pub fn graph_to_mermaid(graph: &ResolutionGraph, strip_root_path: Option<&Path>) -> String {
    let mut mermaid = String::from("flowchart TD\n");
    let mut node_map = BTreeMap::new();

    let path_prefix = strip_root_path.unwrap_or_else(|| Path::new(""));

    // Assign a simple identifier to each node
    for node_idx in graph.node_indices() {
        let id = format!("N{}", node_idx.index());
        node_map.insert(node_idx, id.clone());

        let package = &graph[node_idx];
        let name = &package.identity.name;
        let path = package
            .local_path
            .strip_prefix(path_prefix)
            .unwrap()
            .to_string_lossy();

        let mut identity_str = String::new();
        package
            .identity
            .location
            .fmt_strip_root_path(&mut identity_str, Some(path_prefix))
            .unwrap();
        identity_str = identity_str.replace("://", "_");

        writeln!(
            &mut mermaid,
            "    {}[\"{}<br><br>{}<br><br>{}\"]",
            id, name, identity_str, path
        )
        .unwrap();
    }

    // Add edges
    for edge in graph.edge_references() {
        let source = node_map.get(&edge.source()).unwrap();
        let target = node_map.get(&edge.target()).unwrap();
        writeln!(&mut mermaid, "    {} --> {}", source, target).unwrap();
    }

    mermaid
}

// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{datalog::DatalogBackend, util::make_absolute};

/// Specifies reduction operations that may be performed
/// on a call graph. Supported operations are:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallGraphReduction {
    /// Only include nodes reachable from the given node.
    /// See `CallGraph::filter_reachable`.
    Slice(Box<str>),
    /// Remove nodes in the graph that belong to crates other than
    /// `CallGraphConfig.included_crates`. The outgoing edges of these
    /// removed node are connected to the node's parents.
    /// See `CallGraph::fold_excluded`.
    Fold,
    /// Remove duplicated edges (only considers edge endpoints).
    /// See `CallGraph::deduplicate_edges`.
    Deduplicate,
    /// Remove nodes that have no incoming or outgoing edges.
    /// See `CallGraph::filter_no_edges`.
    Clean,
}

/// Configuration options for Datalog output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatalogConfig {
    /// Specifies location for graph to be output as Datalog input relations.
    pub ddlog_output_path: PathBuf,
    /// Specifies location for mapping from type identifiers to type strings.
    pub type_map_output_path: PathBuf,
    /// Optionally specifies the location for manually defined type relations
    /// to be imported.
    pub type_relations_path: Option<PathBuf>,
    /// Datalog output backend to use.
    /// Currently, Differential Datalog and Soufflé are supported.
    pub datalog_backend: DatalogBackend,
    /// Analysis raw output path
    pub analysis_raw_output_path: PathBuf,
    /// Analysis decoded output path
    pub analysis_decoded_output_path: PathBuf,
}

/// Configuration for node type input relations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeConfig {
    pub entry: Vec<Box<str>>,
    pub checker: Vec<Box<str>>,
    pub safe: Vec<Box<str>>,
    pub exit: Vec<Box<str>>,
}

/// Configuration options for call graph generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphConfig {
    /// Optionally specifies location for graph to be output in dot format
    /// (for Graphviz).
    pub dot_output_path: Option<PathBuf>,
    /// A list of call graph reductions to apply sequentially
    /// to the call graph.
    pub reductions: Vec<CallGraphReduction>,
    /// A list of crates to include in the call graph.
    /// Nodes belonging to crates not in this list will be removed.
    pub included_crates: Vec<Box<str>>,
    /// Datalog output configuration
    pub datalog_config: Option<DatalogConfig>,
    /// Node type annotations
    pub node_types: NodeTypeConfig,
}

/// Generate a complete CallGraphConfig from a combination of the partial config
/// file and command line options
pub fn generate_config(
    config_path: &Path,
    call_graph_only: bool,
    datalog_backend: Option<DatalogBackend>,
    relations_path: Option<PathBuf>,
) -> Result<(CallGraphConfig, PathBuf), String> {
    let output_path = Path::new("./output");
    if !output_path.exists() {
        fs::create_dir(output_path)
            .map_err(|msg| format!("Failed to create output directory: {}", msg))?;
    }
    let mut config: CallGraphConfig = fs::read_to_string(config_path)
        .map_err(|msg| format!("Failed to read config file: {}", msg))
        .and_then(|config_str| {
            serde_json::from_str::<CallGraphConfig>(&config_str)
                .map_err(|msg| format!("Failed to parse config: {}", msg))
        })?;
    let dot_output_path = make_absolute(&output_path.join("graph.dot"))
        .map_err(|msg| format!("Failed to construct dot_output_path: {}", msg))?;
    config.dot_output_path = Some(dot_output_path);
    if !call_graph_only {
        let datalog_backend = match datalog_backend {
            Some(backend) => backend,
            None => DatalogBackend::Souffle,
        };
        let ddlog_output_path = make_absolute(&match datalog_backend {
            DatalogBackend::DifferentialDatalog => output_path.join("graph.dat"),
            DatalogBackend::Souffle => output_path.to_path_buf(),
        })
        .map_err(|msg| format!("Failed to construct ddlog_output_path: {}", msg))?;
        let type_map_output_path = make_absolute(&output_path.join("graph_types.json"))
            .map_err(|msg| format!("Failed to construct type_map_output_path: {}", msg))?;
        let type_relations_path = match relations_path {
            Some(path) => {
                let canonical_path = make_absolute(&path)
                    .map_err(|msg| format!("Failed to construct type_relations_path: {}", msg))?;
                Some(canonical_path)
            }
            None => None,
        };
        let analysis_raw_output_path = match datalog_backend {
            DatalogBackend::DifferentialDatalog => output_path.join("analysis.out"),
            DatalogBackend::Souffle => output_path.to_path_buf(),
        };
        let analysis_decoded_output_path = output_path.join("decoded.json");
        config.datalog_config = Some(DatalogConfig {
            ddlog_output_path,
            type_map_output_path,
            type_relations_path,
            datalog_backend,
            analysis_raw_output_path,
            analysis_decoded_output_path,
        });
    }
    let new_config_path = output_path.join("config.json");
    let config_str = serde_json::to_string(&config)
        .map_err(|msg| format!("Failed to serialize config: {}", msg))?;
    fs::write(new_config_path.clone(), config_str)
        .map_err(|msg| format!("Failed to write config: {}", msg))?;
    Ok((config, new_config_path))
}

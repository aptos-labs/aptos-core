// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    datalog::{
        determine_op_type, DatalogRelation, DatalogRelationOperand, DatalogRelations, NodeType,
    },
    util::make_absolute,
};

// Path to the Soufflé analysis file
const SOUFFLE_ANALYSIS_PATH: &str = "analyses/souffle.dl";

// Run the Soufflé Datalog analysis on the test file
pub fn run_souffle_analysis(analysis_output_path: &Path) -> Result<(), String> {
    let souffle_dl_path = make_absolute(Path::new(SOUFFLE_ANALYSIS_PATH))?;
    Command::new("souffle")
        .current_dir(analysis_output_path)
        .arg(souffle_dl_path.as_os_str())
        .output()
        .map(|_| ())
        .map_err(|msg| format!("Failed to run analysis: {}", msg))
}

/// Parse a Soufflé Datalog Datalog output relation into the
/// format of a DatalogRelation
fn parse_souffle_relations(output_csv: &Path) -> Result<DatalogRelations, String> {
    assert!(output_csv.file_stem().is_some());
    let relation_name = output_csv.file_stem().unwrap();
    let csv_str = fs::read_to_string(output_csv)
        .map_err(|msg| format!("Failed to read output CSV: {}", msg))?;
    let mut rdr = csv::Reader::from_reader(csv_str.as_bytes());
    let headers: csv::StringRecord;
    {
        headers = rdr.headers().unwrap().to_owned();
    }
    let mut relations = DatalogRelations::new();
    for result in rdr.records() {
        let record = result.unwrap();
        let mut operands = Vec::<DatalogRelationOperand>::new();
        for (i, field) in record.iter().enumerate() {
            let op_name = headers.get(i).unwrap().to_owned();
            let op_type = determine_op_type(&op_name)?;
            let operand =
                DatalogRelationOperand::new(op_name, field.parse::<u32>().unwrap(), None, op_type);
            operands.push(operand);
        }
        let relation = DatalogRelation::new(relation_name.to_str().unwrap().to_owned(), operands);
        relations.push(relation);
    }
    Ok(relations)
}

/// Parse all of the Soufflé datalog analysis output relations
pub fn parse_souffle_output(output_path: &Path) -> Result<DatalogRelations, String> {
    let mut output_relation_paths: Vec<PathBuf> = Vec::new();
    assert!(output_path.is_dir());
    for file in output_path.read_dir().unwrap() {
        let _ = file.map(|file_elem| {
            let path = file_elem.path();
            if let Some(extension) = path.extension() {
                if extension == "csv" {
                    output_relation_paths.push(path);
                }
            }
        });
    }
    let mut relations = DatalogRelations::new();
    for path in output_relation_paths {
        let relations_subset = parse_souffle_relations(&path).map_err(|msg| msg)?;
        relations.extend(relations_subset);
    }
    Ok(relations)
}

/// Output node type annotations as datalog input relations
pub fn write_souffle_node_types(node_types: &[NodeType], output_path: &Path) -> Result<(), String> {
    let mut out_strs = Vec::<String>::new();
    for node_type in node_types.iter() {
        out_strs.push(match node_type {
            NodeType::Entry(id) => format!("{},0", id),
            NodeType::Checker(id) => format!("{},1", id),
            NodeType::Safe(id) => format!("{},1", id),
            NodeType::Exit(id) => format!("{},2", id),
        });
    }
    out_strs.sort_unstable();
    let out_str = out_strs.join("\n");
    fs::write(output_path.join("NodeType.facts"), out_str)
        .map(|_| ())
        .map_err(|msg| format!("Failed to write node types: {}", msg))
}

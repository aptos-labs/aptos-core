// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use regex::Regex;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use crate::{
    datalog::{
        determine_op_type, DatalogRelation, DatalogRelationOperand, DatalogRelations, NodeType,
    },
    util::get_child_output,
};

// This path is automatically generated as part of setting up the
// Differential Datalog analysis
const DDLOG_CLI_PATH: &str = "analyses/ddlog_ddlog/target/release/ddlog_cli";

// Run the Differential Datalog analysis on the test file and
// capture output
pub fn run_ddlog_analysis(
    ddlog_output_path: &Path,
    analysis_output_path: &Path,
) -> Result<(), String> {
    let file_data = fs::read_to_string(ddlog_output_path)
        .map_err(|msg| format!("Failed to read dat file: {}", msg))?;
    let mut child = Command::new(DDLOG_CLI_PATH)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|msg| format!("Failed to run analysis: {}", msg))?;
    child
        .stdin
        .take()
        .unwrap()
        .write(file_data.as_bytes())
        .map_err(|msg| format!("Failed to run analysis: {}", msg))?;
    let analysis_output = child
        .wait_with_output()
        .map(|output| get_child_output(&output))
        .map_err(|msg| format!("Failed to run analysis: {}", msg))?;
    fs::write(analysis_output_path, analysis_output)
        .map(|_| ())
        .map_err(|msg| format!("Failed to record analysis results: {}", msg))
}

/// Parse a Differential Datalog Datalog output relation into the
/// format of a DatalogRelation
fn parse_ddlog_relation(line: &str) -> Result<DatalogRelation, String> {
    let relation_name = (match Regex::new(r#"([A-Z])\w+"#).unwrap().captures(line) {
        Some(captures) => Ok(captures[0].to_owned()),
        None => Err("Failed to find relation name"),
    })?;
    let operand_string = (match Regex::new(r#"\{(.*)\}"#).unwrap().captures(line) {
        Some(captures) => Ok(captures[1].to_owned()),
        None => Err("Failed to find relation operands"),
    })?;
    let mut operands = Vec::<DatalogRelationOperand>::new();
    let operand_strs = operand_string.split(',').collect::<Vec<&str>>();
    for operand_str in operand_strs.iter() {
        if let Some(captures) = Regex::new(r#"\.(\w+) = (\d+)"#)
            .unwrap()
            .captures(operand_str)
        {
            assert!(captures.len() == 3);
            let op_name = captures[1].to_owned();
            let op_type = determine_op_type(&op_name)?;
            operands.push(DatalogRelationOperand::new(
                op_name,
                captures[2].to_owned().parse::<u32>().unwrap(),
                None,
                op_type,
            ));
        }
    }
    Ok(DatalogRelation::new(relation_name, operands))
}

/// Parse all of the Differential datalog analysis output relations
pub fn parse_ddlog_output(analysis_output_path: &Path) -> Result<DatalogRelations, String> {
    fs::read_to_string(analysis_output_path)
        .map_err(|msg| format!("Failed to read analysis output: {}", msg))
        .and_then(|out| {
            let lines = out.split('\n').collect::<Vec<&str>>();
            let mut relations = Vec::<DatalogRelation>::new();
            for line in lines.iter().filter(|line| line.contains('{')) {
                match parse_ddlog_relation(line) {
                    Ok(relation) => {
                        relations.push(relation);
                    }
                    Err(msg) => {
                        return Err(format!(
                            "Failed to parse ddlog relation: {}\nbecause: {}",
                            line, msg
                        ));
                    }
                }
            }
            Ok(relations)
        })
}

/// Output node type annotations as datalog input relations
pub fn write_ddlog_node_types(
    node_types: &[NodeType],
    ddlog_relations_path: &Path,
) -> Result<(), String> {
    let mut out_strs = Vec::<String>::new();
    for node_type in node_types.iter() {
        out_strs.push(match node_type {
            NodeType::Entry(id) => format!("insert NodeType({},Entry)", id),
            NodeType::Checker(id) => format!("insert NodeType({},Checker)", id),
            NodeType::Safe(id) => format!("insert NodeType({},Checker)", id),
            NodeType::Exit(id) => format!("insert NodeType({},Exit)", id),
        });
    }
    let mut out_str = out_strs.join(";\n");
    out_str.push_str(";\ncommit;\ndump CheckedType;\ndump NotCheckedType;");
    let ddlog_relations_str = fs::read_to_string(ddlog_relations_path)
        .map_err(|msg| format!("Failed to read ddlog relations: {}", msg))?;
    fs::write(
        ddlog_relations_path,
        ddlog_relations_str.replace("commit;", &out_str),
    )
    .map(|_| ())
    .map_err(|msg| format!("Failed to write node type relations: {}", msg))
}

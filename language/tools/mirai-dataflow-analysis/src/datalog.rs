// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::util::{NodeMap, TypeMap};

/// Annotations for NodeType input relations
#[derive(Debug, Clone)]
pub enum NodeType {
    Entry(u32),
    Checker(u32),
    Safe(u32),
    Exit(u32),
}

pub type DatalogRelations = Vec<DatalogRelation>;

/// A Datalog relation operand is an index of a
/// call graph node or a call graph edge type
#[derive(Debug, Serialize)]
pub enum DatalogRelationOperandType {
    Node,
    Type,
}

/// A Datalog relation operand has a name,
/// index, type, and string representation
#[derive(Debug, Serialize)]
pub struct DatalogRelationOperand {
    name: String,
    index: u32,
    string: Option<String>,
    op_type: DatalogRelationOperandType,
}

impl DatalogRelationOperand {
    pub fn new(
        name: String,
        index: u32,
        string: Option<String>,
        op_type: DatalogRelationOperandType,
    ) -> DatalogRelationOperand {
        DatalogRelationOperand {
            name,
            index,
            string,
            op_type,
        }
    }
}

/// A Datalog relation has a name and a list
/// of operands
#[derive(Debug, Serialize)]
pub struct DatalogRelation {
    name: String,
    operands: Vec<DatalogRelationOperand>,
}

impl DatalogRelation {
    pub fn new(name: String, operands: Vec<DatalogRelationOperand>) -> DatalogRelation {
        DatalogRelation { name, operands }
    }
}

/// The supported Datalog backend are
/// Differential Datalog and Soufflé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatalogBackend {
    DifferentialDatalog,
    Souffle,
}

impl FromStr for DatalogBackend {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DifferentialDatalog" => Ok(DatalogBackend::DifferentialDatalog),
            "Souffle" => Ok(DatalogBackend::Souffle),
            _ => Err(format!("Failed to read Datalog backend: {}", s)),
        }
    }
}

/// Determine the operand type of a Datalog relation from its name
pub fn determine_op_type(op_name: &str) -> Result<DatalogRelationOperandType, String> {
    if op_name.contains("node") || op_name.contains("checker") {
        Ok(DatalogRelationOperandType::Node)
    } else if op_name == "t" {
        Ok(DatalogRelationOperandType::Type)
    } else {
        Err(format!(
            "Failed to determine operand type for node: {}",
            op_name
        ))
    }
}

/// Use the node and type mappings to decode indexes in relation
/// operands to strings
pub fn decode_analysis_output(
    relations: &mut DatalogRelations,
    type_map: &TypeMap,
    node_map: &NodeMap,
) {
    for relation in relations.iter_mut() {
        for operand in relation.operands.iter_mut() {
            match operand.op_type {
                DatalogRelationOperandType::Node => {
                    operand.string = node_map.get(&operand.index).map(|s| s.to_owned());
                }
                DatalogRelationOperandType::Type => {
                    operand.string = type_map.get(&operand.index).map(|s| s.to_owned());
                }
            }
        }
    }
}

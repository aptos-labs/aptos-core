// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::FunctionPass;
use move_binary_format::file_format::{Bytecode, FunctionDefinition};
use move_core_types::identifier::Identifier;
use std::collections::{hash_map::Entry, HashMap, HashSet};

pub struct DistributionInformation {
    num_instructions: usize,
    instruction_distribution: HashMap<Bytecode, usize>,
}

impl DistributionInformation {
    #[allow(dead_code)]
    pub fn num_instructions(&self) -> usize {
        self.num_instructions
    }

    #[allow(dead_code)]
    pub fn num_instructions_filtered<P>(&self, predicate: P) -> usize
    where
        P: Fn(&Bytecode) -> bool,
    {
        self.instruction_distribution
            .iter()
            .filter(|(instruction, _)| predicate(instruction))
            .map(|(_, count)| count)
            .sum()
    }
}

pub struct CollectInstructionDistribution {
    registered_instructions: HashSet<Bytecode>,
    instruction_distribution: HashMap<Identifier, DistributionInformation>,
}

impl CollectInstructionDistribution {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            registered_instructions: HashSet::new(),
            instruction_distribution: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_instructions(mut self, instructions: impl IntoIterator<Item = Bytecode>) -> Self {
        for instruction in instructions {
            self.registered_instructions.insert(instruction);
        }
        self
    }

    #[allow(dead_code)]
    pub fn instruction_distribution_per_module(&self) -> DistributionInformation {
        let mut num_instructions = 0;
        let mut instruction_distribution = HashMap::new();

        for (_, distribution_info) in self.instruction_distribution.iter() {
            for (instruction, count) in &distribution_info.instruction_distribution {
                num_instructions += count;
                match instruction_distribution.entry(instruction.clone()) {
                    Entry::Occupied(mut entry) => {
                        *entry.get_mut() += *count;
                    },
                    Entry::Vacant(entry) => {
                        entry.insert(*count);
                    },
                }
            }
        }

        DistributionInformation {
            num_instructions,
            instruction_distribution,
        }
    }
}

impl FunctionPass for CollectInstructionDistribution {
    fn run_on_function(&mut self, function_name: Identifier, function: &FunctionDefinition) {
        let mut instruction_distribution = self
            .registered_instructions
            .iter()
            .map(|instr| (instr.clone(), 0))
            .collect::<HashMap<Bytecode, usize>>();

        if let Some(code_unit) = &function.code {
            let num_instructions = code_unit.code.len();
            for i in 0..num_instructions {
                let instruction = code_unit.code[i].clone();
                if let Entry::Occupied(mut entry) = instruction_distribution.entry(instruction) {
                    let count = entry.get_mut();
                    *count += 1;
                }
            }
        }

        let code = &function.code;
        let num_instructions = code
            .as_ref()
            .map(|code_unit| code_unit.code.len())
            .unwrap_or(0);
        let info = DistributionInformation {
            num_instructions,
            instruction_distribution,
        };
        self.instruction_distribution.insert(function_name, info);
    }
}

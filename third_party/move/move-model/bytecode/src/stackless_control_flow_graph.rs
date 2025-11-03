// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Adapted from control_flow_graph for Bytecode, this module defines the control-flow graph on
//! Stackless Bytecode used in analysis as part of Move prover.

use crate::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Label},
};
use move_binary_format::file_format::CodeOffset;
use petgraph::{dot::Dot, graph::Graph};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

type Map<K, V> = BTreeMap<K, V>;
type Set<V> = BTreeSet<V>;
pub type BlockId = CodeOffset;

#[derive(Debug)]
struct Block {
    successors: Vec<BlockId>,
    content: BlockContent,
}

#[derive(Copy, Clone, Debug)]
pub enum BlockContent {
    Basic {
        lower: CodeOffset,
        upper: CodeOffset,
    },
    Dummy,
}

pub struct StacklessControlFlowGraph {
    entry_block_id: BlockId,
    blocks: Map<BlockId, Block>,
    backward: bool,
}

const DUMMY_ENTRANCE: BlockId = 0;
const DUMMY_EXIT: BlockId = 1;

impl StacklessControlFlowGraph {
    pub fn new_forward(code: &[Bytecode]) -> Self {
        Self {
            entry_block_id: DUMMY_ENTRANCE,
            blocks: Self::collect_blocks(code),
            backward: false,
        }
    }

    /// If from_all_blocks is false, perform backward analysis only from blocks that may exit.
    /// If from_all_blocks is true, perform backward analysis from all blocks.
    /// Notice that it is guaranteed that forward and backward graphs use the
    /// same block numbering.
    pub fn new_backward(code: &[Bytecode], from_all_blocks: bool) -> Self {
        let blocks = Self::collect_blocks(code);
        let mut block_id_to_predecessors: Map<BlockId, Vec<BlockId>> =
            blocks.keys().map(|block_id| (*block_id, vec![])).collect();
        for (block_id, block) in &blocks {
            for succ_block_id in &block.successors {
                if from_all_blocks && *succ_block_id == DUMMY_EXIT {
                    continue;
                }
                let predecessors = block_id_to_predecessors.get_mut(succ_block_id).unwrap();
                predecessors.push(*block_id);
            }
        }
        if from_all_blocks {
            let predecessors = block_id_to_predecessors.get_mut(&DUMMY_EXIT).unwrap();
            blocks.keys().for_each(|block_id| {
                if *block_id != DUMMY_ENTRANCE && *block_id != DUMMY_EXIT {
                    predecessors.push(*block_id);
                }
            });
        }
        Self {
            entry_block_id: DUMMY_EXIT,
            blocks: block_id_to_predecessors
                .into_iter()
                .map(|(block_id, predecessors)| {
                    (block_id, Block {
                        successors: predecessors,
                        content: blocks[&block_id].content,
                    })
                })
                .collect(),
            backward: true,
        }
    }

    /// Generate a block that contains all the code
    pub fn one_block(code: &[Bytecode]) -> Self {
        let mut blocks = Map::new();
        let mut successors = vec![];
        let entry_bb = 2 as BlockId;
        successors.insert(0, entry_bb);
        successors.insert(1, DUMMY_EXIT);
        let bb = BlockContent::Basic {
            lower: 0,
            upper: code.len() as u16 - 1,
        };
        blocks.insert(entry_bb, Block {
            successors,
            content: bb,
        });
        blocks.insert(DUMMY_ENTRANCE, Block {
            successors: vec![entry_bb],
            content: BlockContent::Dummy,
        });
        blocks.insert(DUMMY_EXIT, Block {
            successors: Vec::new(),
            content: BlockContent::Dummy,
        });
        Self {
            entry_block_id: DUMMY_ENTRANCE,
            blocks,
            backward: false,
        }
    }

    fn collect_blocks(code: &[Bytecode]) -> Map<BlockId, Block> {
        // First go through and collect basic block offsets.
        // Need to do this first in order to handle backwards edges.
        let label_offsets = Bytecode::label_offsets(code);
        let mut bb_offsets = Set::new();
        bb_offsets.insert(0);
        for pc in 0..code.len() {
            StacklessControlFlowGraph::record_block_ids(
                pc as CodeOffset,
                code,
                &mut bb_offsets,
                &label_offsets,
            );
        }
        // Now construct blocks
        let mut blocks = Map::new();
        // Maps basic block entry offsets to their key in blocks
        let mut offset_to_key = Map::new();
        // Block counter starts at 2 because entry and exit will be block 0 and 1
        let mut bcounter = 2;
        let mut block_entry = 0;
        for pc in 0..code.len() {
            let co_pc: CodeOffset = pc as CodeOffset;
            // Create a basic block
            if StacklessControlFlowGraph::is_end_of_block(co_pc, code, &bb_offsets) {
                let mut successors = Bytecode::get_successors(co_pc, code, &label_offsets);
                for successor in successors.iter_mut() {
                    *successor = *offset_to_key.entry(*successor).or_insert(bcounter);
                    bcounter = std::cmp::max(*successor + 1, bcounter);
                }
                if code[co_pc as usize].is_exit() {
                    successors.push(DUMMY_EXIT);
                }
                let bb = BlockContent::Basic {
                    lower: block_entry,
                    upper: co_pc,
                };
                let key = *offset_to_key.entry(block_entry).or_insert(bcounter);
                bcounter = std::cmp::max(key + 1, bcounter);
                blocks.insert(key, Block {
                    successors,
                    content: bb,
                });
                block_entry = co_pc + 1;
            }
        }
        assert_eq!(block_entry, code.len() as CodeOffset);
        let entry_bb = *offset_to_key.get(&0).unwrap();
        blocks.insert(DUMMY_ENTRANCE, Block {
            successors: vec![entry_bb],
            content: BlockContent::Dummy,
        });
        blocks.insert(DUMMY_EXIT, Block {
            successors: Vec::new(),
            content: BlockContent::Dummy,
        });
        blocks
    }

    fn is_end_of_block(pc: CodeOffset, code: &[Bytecode], block_ids: &Set<BlockId>) -> bool {
        pc + 1 == (code.len() as CodeOffset) || block_ids.contains(&(pc + 1))
    }

    fn record_block_ids(
        pc: CodeOffset,
        code: &[Bytecode],
        block_ids: &mut Set<BlockId>,
        label_offsets: &BTreeMap<Label, CodeOffset>,
    ) {
        let bytecode = &code[pc as usize];

        for label in bytecode.branch_dests() {
            block_ids.insert(*label_offsets.get(&label).unwrap());
        }

        if bytecode.is_branching() && pc + 1 < (code.len() as CodeOffset) {
            block_ids.insert(pc + 1);
        }
    }
}

impl StacklessControlFlowGraph {
    pub fn successors(&self, block_id: BlockId) -> &Vec<BlockId> {
        &self.blocks[&block_id].successors
    }

    /// Returns a map from a block to a vector of its successors
    pub fn get_successors_map(&self) -> BTreeMap<BlockId, Vec<BlockId>> {
        self.blocks
            .iter()
            .map(|(block_id, block)| (*block_id, block.successors.clone()))
            .collect()
    }

    pub fn content(&self, block_id: BlockId) -> &BlockContent {
        &self.blocks[&block_id].content
    }

    pub fn code_range(&self, block_id: BlockId) -> Range<usize> {
        match self.content(block_id) {
            BlockContent::Basic { lower, upper } => *lower as usize..(*upper as usize + 1),
            BlockContent::Dummy => 0..0,
        }
    }

    pub fn blocks(&self) -> Vec<BlockId> {
        self.blocks.keys().cloned().collect()
    }

    pub fn reachable_blocks(
        &self,
        from: BlockId,
        mut edge_filter: impl FnMut(BlockId, BlockId) -> bool,
    ) -> BTreeSet<BlockId> {
        let mut result = BTreeSet::new();
        let mut todo = vec![from];
        while let Some(blk_id) = todo.pop() {
            if result.insert(blk_id) {
                for succ in self.successors(blk_id) {
                    if edge_filter(blk_id, *succ) {
                        todo.push(*succ)
                    }
                }
            }
        }
        result
    }

    pub fn entry_block(&self) -> BlockId {
        self.entry_block_id
    }

    pub fn exit_block(&self) -> BlockId {
        if self.backward {
            DUMMY_ENTRANCE
        } else {
            DUMMY_EXIT
        }
    }

    pub fn enclosing_block(&self, code_offset: CodeOffset) -> BlockId {
        for blk_id in self.blocks.keys() {
            if self.code_range(*blk_id).contains(&(code_offset as usize)) {
                return *blk_id;
            }
        }
        panic!("code offset not in the control flow graph")
    }

    pub fn instr_indexes(
        &self,
        block_id: BlockId,
    ) -> Option<Box<dyn DoubleEndedIterator<Item = CodeOffset>>> {
        match self.blocks[&block_id].content {
            BlockContent::Basic { lower, upper } => Some(Box::new(lower..=upper)),
            BlockContent::Dummy => None,
        }
    }

    pub fn instr_offset_bounds(&self, block_id: BlockId) -> Option<(CodeOffset, CodeOffset)> {
        match self.blocks[&block_id].content {
            BlockContent::Basic { lower, upper } => Some((lower, upper)),
            BlockContent::Dummy => None,
        }
    }

    pub fn num_blocks(&self) -> u16 {
        self.blocks.len() as u16
    }

    pub fn is_dummy(&self, block_id: BlockId) -> bool {
        matches!(self.blocks[&block_id].content, BlockContent::Dummy)
    }

    pub fn display(&self) {
        println!("+=======================+");
        println!("entry_block_id = {}", self.entry_block_id);
        println!("blocks = {:?}", self.blocks);
        println!("is_backward = {}", self.backward);
        println!("+=======================+");
    }
}

/// Iterator over blocks of a control flow graph in DFS order
/// (always choosing the left-most child to visit first).
pub struct DFSLeft<'a> {
    successors: &'a BTreeMap<BlockId, Vec<BlockId>>,
    // blocks scheduled to visit
    to_visit: Vec<BlockId>,
    // blocks visited
    visited: BTreeSet<BlockId>,
    // blocks unvisited, used only when iterating over all blocks
    unvisited: Option<BTreeSet<BlockId>>,
}

impl<'a> DFSLeft<'a> {
    /// Create DFSLeft iterator starting from a specified start block
    /// `successors`: should have all blocks as keys, even if they have no successors
    /// `visit_all`: whether to visit all blocks or just blocks reachable from the start block
    pub fn new(
        successors: &'a BTreeMap<BlockId, Vec<BlockId>>,
        start: BlockId,
        visit_all: bool,
    ) -> Self {
        let to_visit = vec![start];
        Self {
            successors,
            to_visit,
            visited: BTreeSet::new(),
            unvisited: if visit_all {
                Some(successors.keys().cloned().collect())
            } else {
                None
            },
        }
    }

    /// Gets the next block scheduled, but it may have been visited when scheduled by multiple predecessors
    fn get_next_to_visit(&mut self) -> Option<BlockId> {
        if let Some(to_visit) = self.to_visit.pop() {
            Some(to_visit)
        } else if let Some(unvisited_blocks) = &mut self.unvisited {
            unvisited_blocks.pop_first()
        } else {
            None
        }
    }

    /// Gets the next block to visit
    fn get_next_unvisited(&mut self) -> Option<BlockId> {
        let next_to_visit = self.get_next_to_visit()?;
        if self.visited.contains(&next_to_visit) {
            // we won't run into a infinite loop
            // because eventually we will run out of scheduled blocks or unvisited blocks
            // by calling `get_next_to_visit`
            self.get_next_unvisited()
        } else {
            Some(next_to_visit)
        }
    }
}

impl Iterator for DFSLeft<'_> {
    type Item = BlockId;

    fn next(&mut self) -> Option<Self::Item> {
        let visiting = self.get_next_unvisited()?;
        self.visited.insert(visiting);
        if let Some(unvisited) = &mut self.unvisited {
            unvisited.remove(&visiting);
        }
        for suc_block in self
            .successors
            .get(&visiting)
            .expect("successors")
            .iter()
            .rev()
        {
            self.to_visit.push(*suc_block);
        }
        Some(visiting)
    }
}

// CFG dot graph generation
struct DotCFGBlock<'env> {
    block_id: BlockId,
    content: BlockContent,
    func_target: &'env FunctionTarget<'env>,
    label_offsets: BTreeMap<Label, CodeOffset>,
}

impl std::fmt::Display for DotCFGBlock<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let code_range = match self.content {
            BlockContent::Basic { lower, upper } => format!("offset {}..={}", lower, upper),
            BlockContent::Dummy => "X".to_owned(),
        };
        writeln!(f, "[Block {} - {}]", self.block_id, code_range)?;
        match self.content {
            BlockContent::Basic { lower, upper } => {
                let code = &self.func_target.data.code;
                for (offset, instruction) in
                    (lower..=upper).zip(&code[(lower as usize)..=(upper as usize)])
                {
                    let text = self.func_target.pretty_print_bytecode(
                        &self.label_offsets,
                        offset as usize,
                        instruction,
                        false,
                    );
                    writeln!(f, "{}", text)?;
                }
            },
            BlockContent::Dummy => {},
        }
        Ok(())
    }
}

// A dummy struct to implement fmt required by petgraph::Dot
struct DotCFGEdge {}

impl std::fmt::Display for DotCFGEdge {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Ok(())
    }
}

/// Generate the dot representation of the CFG (which can be rendered by the Dot program)
pub fn generate_cfg_in_dot_format<'env>(
    func_target: &'env FunctionTarget<'env>,
    forward: bool,
) -> String {
    let code = &func_target.data.code;
    let cfg = if forward {
        StacklessControlFlowGraph::new_forward(code)
    } else {
        StacklessControlFlowGraph::new_backward(code, true)
    };
    let label_offsets = Bytecode::label_offsets(code);
    let mut graph = Graph::new();

    // add nodes
    let mut node_map = Map::new();
    for (block_id, block) in &cfg.blocks {
        let dot_block = DotCFGBlock {
            block_id: *block_id,
            content: block.content,
            func_target,
            label_offsets: label_offsets.clone(),
        };
        let node_index = graph.add_node(dot_block);
        node_map.insert(block_id, node_index);
    }

    // add edges
    for (block_id, block) in &cfg.blocks {
        for successor in &block.successors {
            graph.add_edge(
                *node_map.get(block_id).unwrap(),
                *node_map.get(successor).unwrap(),
                DotCFGEdge {},
            );
        }
    }

    // generate dot string
    format!(
        "{}",
        Dot::with_attr_getters(&graph, &[], &|_, _| "".to_string(), &|_, _| {
            "shape=box".to_string()
        })
    )
}

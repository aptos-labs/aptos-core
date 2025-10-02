// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a stackless-bytecode security check that looks for
//! functions containing possible infinite recursion, either by themselves, or
//! infinite recursive cycles with any number of other functions. I.e. f() calls
//! g(), which calls h(), which calls i(), which calls f() again.

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::model::{FunId, GetNameString, GlobalEnv, Loc, ModuleId};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::{BlockContent, BlockId, StacklessControlFlowGraph},
};
use once_cell::sync::Lazy;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::Mutex,
};

#[derive(Clone)]
struct CallGraphNode {
    //The set of functions that will be unconditionally called by the function.
    callees: HashSet<Function>,
    //Axiliary data.
    visited: bool,
}

impl CallGraphNode {
    fn new(callees: HashSet<Function>) -> CallGraphNode {
        CallGraphNode {
            callees,
            visited: false,
        }
    }
}

//State persistent accross invocations of InfiniteRecursion::check().
struct GlobalState {
    functions: HashMap<Function, CallGraphNode>,
}

static GLOBAL_STATE: Lazy<Mutex<GlobalState>> = Lazy::new(|| Mutex::new(GlobalState::new()));

pub struct InfiniteRecursion {}

impl StacklessBytecodeChecker for InfiniteRecursion {
    fn get_name(&self) -> String {
        "infinite_recursion".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        GlobalState::analyze_target(|e, l, m| self.report(e, l, m), target);
    }
}

impl GlobalState {
    pub fn new() -> GlobalState {
        GlobalState {
            functions: HashMap::new(),
        }
    }

    //Analyzes a FunctionTarget and internally accumulates information about
    //visited functions. When a cycle of functions that unconditionally call
    //each other is found, it reports it using the provided `report` function.
    pub fn analyze_target<F: Fn(&GlobalEnv, &Loc, &str)>(report: F, target: &FunctionTarget) {
        let mut lock = GLOBAL_STATE.lock().unwrap();

        let mut program = Program::new(target.get_bytecode());
        //program.print();
        let function = Function {
            module: target.module_env().get_id(),
            function: target.get_id(),
        };
        lock.functions.insert(
            function.clone(),
            CallGraphNode::new(program.get_unconditional_calls()),
        );
        lock.find_cycles(report, target, function);
    }

    //Entry function for recursive traverse_call_graph().
    fn find_cycles<F: Fn(&GlobalEnv, &Loc, &str)>(
        &mut self,
        report: F,
        target: &FunctionTarget,
        initial: Function,
    ) {
        //Reset auxiliary data.
        for v in self.functions.values_mut() {
            v.visited = false;
        }

        let mut stack = Vec::new();
        stack.push(initial.clone());
        self.traverse_call_graph(&report, target, &mut stack);
    }

    //Traverses the call graph defined by GlobalState::functions looking for cycles.
    fn traverse_call_graph<F: Fn(&GlobalEnv, &Loc, &str)>(
        &mut self,
        report: &F,
        target: &FunctionTarget,
        stack: &mut Vec<Function>,
    ) {
        if let Some(top) = stack.last().cloned() {
            if let Some(node) = self.functions.get_mut(&top) {
                if node.visited {
                    let env = target.global_env();
                    let cycle = Self::report_infinite_recursion(stack, env);
                    report(
                        env,
                        &target.get_loc(),
                        &format!("Unconditional infinite recursion detected: {}", cycle),
                    );
                    return;
                }
                node.visited = true;
            }

            if let Some(node) = self.functions.get(&top).cloned() {
                for call in node.callees.iter() {
                    stack.push(call.clone());
                    self.traverse_call_graph(report, target, stack);
                    stack.pop();
                }
            }

            if let Some(node) = self.functions.get_mut(&top) {
                node.visited = false;
            }
        }
    }

    fn report_infinite_recursion(stack: &[Function], env: &GlobalEnv) -> String {
        debug_assert!(!stack.is_empty());
        let mut ret = String::new();
        if let Some(position) = stack.iter().position(|x| x == stack.last().unwrap()) {
            let mut first = true;
            for i in stack.iter().skip(position) {
                if first {
                    first = false;
                } else {
                    ret += " -> ";
                }
                ret += &i.get_name(env);
            }
        }
        ret
    }
}

//Represents a block. A "block" is a sequence of opcodes that does not have
//any opcodes that affect control flow, except at the end.
#[derive(Clone, Debug)]
struct Block {
    //The next blocks that may get executed after this one.
    successors: Vec<BlockId>,
    //These are the functions that are called (unconditionally) within the
    //block.
    calls: HashSet<Function>,
    ends_in_ret: bool,

    //Auxiliary data.
    visited: bool,
}

//Uniquely represents a function by module and function ID.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct Function {
    //Module ID.
    pub module: ModuleId,
    //Function ID.
    pub function: FunId,
}

impl Function {
    //Get the fully-qualified function name.
    fn get_name(&self, env: &GlobalEnv) -> String {
        self.module
            .qualified(self.function)
            .get_name_for_display(env)
            .to_string()
    }
}

#[derive(Debug)]
enum StackAction {
    WindUp(BlockId),
    Backtrack(BlockId),
}

//A reduced representation of a program.
struct Program {
    //The program's `Segment`s by their `SegmentId`.
    blocks_map: HashMap<BlockId, Block>,
    //The program's entry point.
    entry: BlockId,
}

impl Program {
    pub fn new(code: &[Bytecode]) -> Program {
        let cfg = StacklessControlFlowGraph::new_forward(code);
        Program {
            blocks_map: Self::construct_block_map(code, &cfg),
            entry: cfg.entry_block(),
        }
    }

    fn construct_block_map(
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
    ) -> HashMap<BlockId, Block> {
        let mut ret = HashMap::new();
        let mut stack = vec![cfg.entry_block()];
        let successors = cfg.get_successors_map();

        while let Some(current) = stack.pop() {
            if ret.contains_key(&current) {
                continue;
            }

            let successors = if let Some(successors) = successors.get(&current) {
                successors.clone()
            } else {
                vec![]
            };
            for i in successors.iter() {
                stack.push(*i);
            }

            let (begin, end) = if let BlockContent::Basic { lower, upper } = cfg.content(current) {
                (*lower as usize, (*upper as usize) + 1)
            } else {
                (0, 0)
            };

            let block = Block {
                successors,
                calls: Self::collect_calls(code, begin, end),
                visited: false,
                ends_in_ret: begin != end && matches!(code.get(end - 1), Some(Bytecode::Ret(_, _))),
            };
            ret.insert(current, block);
        }

        ret
    }

    fn collect_calls(code: &[Bytecode], begin: usize, end: usize) -> HashSet<Function> {
        let mut ret = HashSet::new();
        if end <= begin {
            return ret;
        }

        for opcode in code.iter().skip(begin).take(end - begin) {
            match opcode {
                Bytecode::Call(_, _, Operation::Function(m, f, _), _, _)
                | Bytecode::Call(_, _, Operation::Closure(m, f, _, _), _, _) => {
                    ret.insert(Function {
                        module: *m,
                        function: *f,
                    });
                },
                _ => {},
            }
        }
        ret
    }

    //Find all control flow graph nodes that terminate in a return.
    fn list_exits(&self) -> Vec<BlockId> {
        self.blocks_map
            .iter()
            .filter(|(_, block)| block.ends_in_ret)
            .map(|(id, _)| *id)
            .collect()
    }

    //Use DFS to find any path through the control flow graph that connects two
    //nodes, if any exists.
    fn find_any_path(&mut self, from: &BlockId, to: &BlockId) -> Option<Vec<BlockId>> {
        //Clear auxiliary data.
        for block in self.blocks_map.values_mut() {
            block.visited = false;
        }

        let mut ret = Vec::new();
        let mut found = false;
        let mut stack = vec![StackAction::WindUp(*from)];
        while let Some(action) = stack.pop() {
            let continue_looping = match action {
                StackAction::WindUp(current) => {
                    self.windup(&current, to, &mut stack, &mut found, &mut ret)
                },
                StackAction::Backtrack(current) => self.backtrack(&current, &mut ret),
            };

            if !continue_looping {
                break;
            }
        }

        if found {
            Some(ret)
        } else {
            None
        }
    }

    fn windup(
        &mut self,
        current: &BlockId,
        to: &BlockId,
        stack: &mut Vec<StackAction>,
        found: &mut bool,
        accum: &mut Vec<BlockId>,
    ) -> bool {
        if let Some(data) = self.blocks_map.get_mut(current) {
            if data.visited {
                return true;
            }
            data.visited = true;
            accum.push(*current);

            if current == to {
                *found = true;
                return false;
            }

            stack.push(StackAction::Backtrack(*current));
            if data.successors.is_empty() {
                return true;
            }

            for next in data.successors.iter() {
                stack.push(StackAction::WindUp(*next));
            }
        }

        true
    }

    fn backtrack(&mut self, current: &BlockId, accum: &mut Vec<BlockId>) -> bool {
        if let Some(data) = self.blocks_map.get_mut(current) {
            data.visited = false;
        }

        let top = accum.pop();
        debug_assert!(top == Some(*current));

        true
    }

    //Get all unique functions that are called along a sequence of blocks.
    fn get_calls_for_path(&self, path: &Option<Vec<BlockId>>) -> Option<HashSet<Function>> {
        if let Some(path) = path {
            let mut ret = HashSet::new();

            for x in path.iter() {
                if let Some(data) = self.blocks_map.get(x) {
                    ret.extend(data.calls.iter().cloned());
                }
            }

            Some(ret)
        } else {
            None
        }
    }

    //Find the functions that are unconditionally called by the program.
    //Algorithm:
    // 1. Find all program segments that end in a return.
    // 2. For each of those, find any path from the entry point to them.
    // 3. For each of those paths, accumulate (through unions) a set of all
    //    functions that are called.
    // 4. The result for get_unconditional_calls() is the intersection of all
    //    the sets found during step #3.
    pub fn get_unconditional_calls(&mut self) -> HashSet<Function> {
        let mut ret: Option<HashSet<Function>> = None;

        for exit in self.list_exits() {
            let path = self.find_any_path(&self.entry.clone(), &exit);
            let ucs = self.get_calls_for_path(&path);

            if let Some(ucs) = ucs {
                ret = Some(
                    if let Some(ret2) = ret {
                        ret2.intersection(&ucs).cloned().collect::<HashSet<_>>()
                    } else {
                        ucs
                    },
                );
            }
        }

        ret.unwrap_or_default()
    }

    #[allow(dead_code)]
    fn print(&self) {
        eprintln!("Program{{");
        for (k, v) in self.blocks_map.iter() {
            eprintln!("    [{:?}] = {:?},", k, v);
        }
        eprintln!("}}");
    }
}

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
    stackless_bytecode::{Bytecode, Label, Operation},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Deref,
    sync::{Arc, Mutex},
};

//State persistent accross invocations of InfiniteRecursion::check().
struct GlobalState {
    //key: A function.
    //value: A set of functions that will be unconditionally called by the key.
    functions: HashMap<Function, HashSet<Function>>,
}

static GLOBAL_STATE: Mutex<Option<Arc<Mutex<GlobalState>>>> = Mutex::new(None);

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
    fn new() -> GlobalState {
        GlobalState {
            functions: HashMap::new(),
        }
    }

    fn get_state() -> Arc<Mutex<GlobalState>> {
        let mut gs = GLOBAL_STATE.lock().unwrap();
        match gs.deref() {
            None => {
                let ret =
                    Arc::<Mutex<GlobalState>>::new(Mutex::<GlobalState>::new(GlobalState::new()));
                *gs = Some(ret.clone());
                ret
            },
            Some(p) => p.clone(),
        }
    }

    //Analyzes a FunctionTarget and internally accumulates information about
    //visited functions. When a cycle of functions that unconditionally call
    //each other is found, it reports it using the provided `report` function.
    pub fn analyze_target<F: Fn(&GlobalEnv, &Loc, &str)>(report: F, target: &FunctionTarget) {
        let gs = Self::get_state();
        let mut lock = gs.lock().unwrap();

        let program = Program::new(target.get_bytecode());
        let function = Function {
            module: target.module_env().get_id(),
            function: target.get_id(),
        };
        let calls = program.get_unconditional_calls();
        lock.functions.insert(function.clone(), calls);
        lock.find_cycles(report, target, function);
    }

    //Entry function for recursive traverse_call_graph().
    fn find_cycles<F: Fn(&GlobalEnv, &Loc, &str)>(
        &self,
        report: F,
        target: &FunctionTarget,
        initial: Function,
    ) {
        let mut stack = Vec::<Function>::new();
        stack.push(initial.clone());
        self.traverse_call_graph(&report, target, &mut stack);
    }

    //Traverses the call graph defined by GlobalState::functions looking for cycles.
    fn traverse_call_graph<F: Fn(&GlobalEnv, &Loc, &str)>(
        &self,
        report: &F,
        target: &FunctionTarget,
        stack: &mut Vec<Function>,
    ) {
        if let Some(top) = stack.last() {
            if let Some(calls) = self.functions.get(top) {
                for call in calls.iter() {
                    if let Some(index) = stack.iter().position(|x| x == call) {
                        let env = target.global_env();
                        let cycle = Self::report_infinite_recursion(stack, call, index, env);
                        report(
                            env,
                            &target.get_loc(),
                            &format!("Unconditional infinite recursion detected: {}", cycle),
                        );
                        continue;
                    }
                    stack.push(call.clone());
                    self.traverse_call_graph(report, target, stack);
                    stack.pop();
                }
            }
        }
    }

    fn report_infinite_recursion(
        stack: &[Function],
        f: &Function,
        position: usize,
        env: &GlobalEnv,
    ) -> String {
        let mut ret = String::new();
        for i in stack.iter().skip(position) {
            ret += &i.get_name(env);
            ret += " -> ";
        }
        ret += &f.get_name(env);
        ret
    }
}

//Identifies a segment. See `Segment`.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Copy)]
enum SegmentId {
    //The segment starts at the function's entry point.
    EntryPoint,
    //The segment starts anywhere else other than at the entry point.
    Label(move_stackless_bytecode::stackless_bytecode::Label),
}

//Represents how a segment terminates.
#[derive(Clone, Debug)]
enum Terminator {
    //The segment terminates in an opcode that does not affect control flow, so
    //execution continues to the segment that appears immediately after in the
    //program.
    Next(SegmentId),
    //The segment terminates in an unconditional jump to the specified
    //SegmentId. For the purposes of call graph analysis, this is equivalent to
    //Next().
    Jump(SegmentId),
    //The segment terminates in a conditional jump. The two segment IDs are the
    //true and false branches, although the order is unspecified.
    Branch(SegmentId, SegmentId),
    //The segment terminates in a function return.
    Return,
}

//Represents a segment. A "segment" is a sequence of opcodes that does not have
//any opcodes that affect control flow, except calls.
#[derive(Clone, Debug)]
struct Segment {
    //The offset of the first opcode.
    start: usize,
    //The opcode count for the segment, including the terminator opcode.
    length: usize,
    //The terminator stores the next segment or segments that may be executed by
    //the program, if any.
    term: Terminator,
}

impl Segment {
    //Gets the functions that are called in a segment. Because a segment does
    //not contain control flow opcodes, these functions are called
    //unconditionally by the segment (up to aborts).
    fn get_calls(&self, code: &[Bytecode]) -> HashSet<Function> {
        let mut ret = HashSet::new();
        for instr in code.iter().skip(self.start).take(self.length) {
            if let Bytecode::Call(_, _, Operation::Function(m, f, _), _, _) = instr {
                ret.insert(Function {
                    module: *m,
                    function: *f,
                });
            }
        }
        ret
    }

    pub fn get_next_segments(&self) -> Vec<SegmentId> {
        match &self.term {
            Terminator::Return => vec![],
            Terminator::Next(id) | Terminator::Jump(id) => vec![*id],
            Terminator::Branch(l, r) => vec![*l, *r],
        }
    }
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

//A reduced representation of a program.
struct Program {
    //The program's code as a simple vector of opcodes.
    code: Vec<Bytecode>,
    //The program's `Segment`s by their `SegmentId`.
    segment_map: HashMap<SegmentId, Segment>,
}

//Context information for path-finding.
#[derive(Clone, Copy)]
struct ShortestPathData {
    //The best previous node in the candidate path.
    pub last: Option<SegmentId>,
    //The distance to the starting node.
    pub depth: u64,
}

impl ShortestPathData {
    fn new(last: Option<SegmentId>, depth: u64) -> ShortestPathData {
        ShortestPathData { last, depth }
    }
}

impl Program {
    pub fn new(code: &[Bytecode]) -> Program {
        let mut segmenter = Segmenter::new();
        segmenter.process(code);
        Program {
            code: code.to_vec(),
            segment_map: segmenter.get(),
        }
    }

    //Returns the functions called by every segment. As explained in
    //`Segment::get_calls()`, each HashSet<Function> contains the functions that
    //are unconditionally called by the corresponding segment.
    fn process_unconditional_calls_by_segment(&self) -> HashMap<SegmentId, HashSet<Function>> {
        let mut ret = HashMap::new();
        for (id, segment) in self.segment_map.iter() {
            ret.insert(*id, segment.get_calls(&self.code));
        }
        ret
    }

    //Find all control flow graph nodes that terminate in a return.
    fn list_exits(&self) -> Vec<SegmentId> {
        self.segment_map
            .iter()
            .filter(|(_, segment)| matches!(segment.term, Terminator::Return))
            .map(|(id, _)| *id)
            .collect()
    }

    //Find the shortest path through the control flow graph that connects two
    //nodes of that graph, if any such exists.
    fn find_shortest_path(&self, from: &SegmentId, to: &SegmentId) -> Option<Vec<SegmentId>> {
        let mut data = HashMap::<SegmentId, ShortestPathData>::new();
        let mut queue = VecDeque::<SegmentId>::new();
        data.insert(*from, ShortestPathData::new(None, 0));
        queue.push_back(*from);
        while let Some(id) = queue.pop_front() {
            if let Some(node_data) = data.get(&id).cloned() {
                if let Some(segment) = self.segment_map.get(&id) {
                    for child_id in segment.get_next_segments() {
                        let push = if let Some(child_data) = data.get(&child_id) {
                            child_data.depth > node_data.depth + 1
                        } else {
                            true
                        };

                        if push {
                            data.insert(
                                child_id,
                                ShortestPathData::new(Some(id), node_data.depth + 1),
                            );
                            queue.push_back(child_id);
                        }
                    }
                }
            } else {
                return None;
            }
        }

        let mut ret = Vec::new();

        let mut current = Some(*to);
        while let Some(current2) = current {
            ret.push(current2);
            if let Some(node_data) = data.get(&current2) {
                current = node_data.last;
            } else {
                return None;
            }
        }

        ret.reverse();

        Some(ret)
    }

    //Get all unique functions that are called along a sequence of segments.
    fn get_calls_for_path(
        unconditional_calls: &HashMap<SegmentId, HashSet<Function>>,
        path: &Option<Vec<SegmentId>>,
    ) -> Option<HashSet<Function>> {
        if let Some(path) = path {
            let mut ret = HashSet::new();

            for x in path.iter() {
                if let Some(ucs) = unconditional_calls.get(x) {
                    ret = ret.union(ucs).cloned().collect::<HashSet<_>>();
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
    // 2. For each of those, by hopping from one segment to the next, find the
    //    shortest path from the entry point to it.
    // 3. For each of those paths, determine the set of functions that are
    //    unconditionally called. That would be the union of all the functions
    //    called by each segment along the path.
    // 4. The result for get_unconditional_calls() is the intersection of all
    //    the sets found during step #3.
    pub fn get_unconditional_calls(&self) -> HashSet<Function> {
        let unconditional_calls = self.process_unconditional_calls_by_segment();

        let mut ret: Option<HashSet<Function>> = None;

        for exit in self.list_exits() {
            let path = self.find_shortest_path(&SegmentId::EntryPoint, &exit);
            let ucs = Self::get_calls_for_path(&unconditional_calls, &path);
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
        for (k, v) in self.segment_map.iter() {
            eprintln!("    [{:?}] = {:?},", k, v);
        }
        eprintln!("}}");
    }
}

//Stores working information to split a program into `Segment`s.
struct Segmenter {
    //Map containing the segmented program.
    result: HashMap<SegmentId, Segment>,
    //Indicates whether the current opcode is dead code. Dead code is not
    //included in any segment in `result`.
    dead: bool,
    //The last label that was seen.
    current_label: Option<Label>,
    //The offset of `current_label` in the program.
    current: usize,
    //The offset of the opcode currently being analyzed.
    i: usize,
}

impl Segmenter {
    pub fn new() -> Segmenter {
        Segmenter {
            result: HashMap::new(),
            dead: false,
            current: 0,
            current_label: None,
            i: 0,
        }
    }

    //Splits a sequence of opcodes into `Segment`s.
    pub fn process(&mut self, code: &[Bytecode]) {
        for instr in code.iter() {
            match instr {
                Bytecode::Label(_, label) => {
                    self.push(Terminator::Next(SegmentId::Label(*label)));
                    self.dead = false;
                    self.current = self.i;
                    self.current_label = Some(*label);
                },
                Bytecode::Jump(_, label) => {
                    self.push(Terminator::Jump(SegmentId::Label(*label)));
                    self.dead = true;
                },
                Bytecode::Branch(_, first, second, _) => {
                    self.push(Terminator::Branch(
                        SegmentId::Label(*first),
                        SegmentId::Label(*second),
                    ));
                    self.dead = true;
                },
                Bytecode::Ret(_, _) => {
                    self.push(Terminator::Return);
                    self.dead = true;
                },
                _ => {},
            }
            self.i += 1;
        }
    }

    //Adds a new `Segment` and its ID to `result`. Dead code is ignored.
    fn push(&mut self, term: Terminator) {
        if self.dead {
            return;
        }

        let segment = Segment {
            start: self.current,
            length: self.i - self.current,
            term,
        };

        let id = match self.current_label {
            None => SegmentId::EntryPoint,
            Some(x) => SegmentId::Label(x),
        };
        self.result.insert(id, segment);
    }

    //Extracts the segmented program, consuming `self`.
    pub fn get(self) -> HashMap<SegmentId, Segment> {
        self.result
    }
}

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
    collections::{HashMap, HashSet},
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
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
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

//Stores the accumulated segments and functions that have been executed by
//following a specific path through the program. A program route need not finish
//in a return; it is a working data structure.
#[derive(Clone)]
struct ProgramRoute {
    //The ID of the segment up to where analysis of this route has reached.
    pub current_id: SegmentId,
    //The functions that have been called along this route so far.
    pub calls: HashSet<Function>,
    //The segments that have been visited along this route so far, and how many
    //times.
    pub visited: HashMap<SegmentId, usize>,
}

impl ProgramRoute {
    fn new() -> ProgramRoute {
        ProgramRoute {
            current_id: SegmentId::EntryPoint,
            calls: HashSet::new(),
            visited: HashMap::new(),
        }
    }
}

//A reduced representation of a program.
struct Program {
    //The program's code as a simple vector of opcodes.
    code: Vec<Bytecode>,
    //The program's `Segment`s by their `SegmentId`.
    segment_map: HashMap<SegmentId, Segment>,
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
            ret.insert(id.clone(), segment.get_calls(&self.code));
        }
        ret
    }

    //Follows control flow to find the functions that are unconditionally called
    //by the program.
    pub fn get_unconditional_calls(&self) -> HashSet<Function> {
        let unconditional_calls = self.process_unconditional_calls_by_segment();

        let mut ret: Option<HashSet<Function>> = None;

        //Stack to recursively walk the program graph.
        let mut stack = Vec::<ProgramRoute>::new();
        stack.push(ProgramRoute::new());
        while let Some(mut route) = stack.pop() {
            let id = route.current_id;

            if let Some(visits) = route.visited.get(&id) {
                if *visits > 1 {
                    continue;
                }
                route.visited.insert(id.clone(), *visits + 1);
            } else {
                route.visited.insert(id.clone(), 1);
                if let Some(calls) = unconditional_calls.get(&id) {
                    route.calls = route.calls.union(calls).cloned().collect::<HashSet<_>>();
                }
            };

            if let Some(segment) = self.segment_map.get(&id) {
                match segment.term.clone() {
                    //If the segment terminates in a non-control opcode or in an
                    //unconditional jump, just push it back on the recursive
                    //stack unchanged.
                    Terminator::Next(segment_id) | Terminator::Jump(segment_id) => {
                        route.current_id = segment_id;
                        stack.push(route);
                    },
                    //If the segment terminates in a conditional jump, we need
                    //to follow both branches independently, so clone the
                    //`ProgramRoute` and assign to each, each branch of the
                    //jump.
                    Terminator::Branch(left, right) => {
                        route.current_id = left;
                        stack.push(route.clone());
                        route.current_id = right;
                        stack.push(route);
                    },
                    //If the segment terminates in a return then we're done with
                    //this `ProgramRoute`, so just merge it with `ret`.
                    Terminator::Return => {
                        if let Some(set) = ret {
                            ret = Some(
                                set.intersection(&route.calls)
                                    .cloned()
                                    .collect::<HashSet<_>>(),
                            );
                        } else {
                            ret = Some(route.calls);
                        }
                    },
                }
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

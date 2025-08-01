// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a stackless-bytecode linter that checks for mutable references
//! that are never used mutably, and suggests to use immutable references instead.
//! For example, if a mutable reference is never written to or passed as a mutable reference
//! parameter to a function call, or is not returned as a mutable reference, it can be
//! replaced with an immutable reference.
//!
//! Currently, we only track mutable references that are:
//! - function parameters,
//! - obtained via `&mut` or `borrow_global_mut`.

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::{
    model::{
        FunId,
        GetNameString,
        GlobalEnv,
        Loc,
        ModuleId,
    },
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Label, Operation},
};
use std::{
    collections::{
        HashMap,
        HashSet,
    },
    ops::Deref,
    sync::{
        Arc,
        Mutex
    }
};

struct GlobalState{
    functions: HashMap<Function, HashSet<Function>>,
}

static GLOBAL_STATE: Mutex<Option<Arc<Mutex<GlobalState>>>> = Mutex::new(None);

/// Linter for detecting needless mutable references.
pub struct InfiniteRecursion {}

impl StacklessBytecodeChecker for InfiniteRecursion {
    fn get_name(&self) -> String {
        "infinite_recursion".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        GlobalState::analyze_target(|e, l, m| self.report(e, l, m), target);
    }
}

impl GlobalState{
    fn new() -> GlobalState{
        GlobalState{
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
            }
            Some(p) => p.clone(),
        }
    }
    pub fn analyze_target<F: Fn(&GlobalEnv, &Loc, &str) -> ()>(report: F, target: &FunctionTarget){
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
    fn find_cycles<F: Fn(&GlobalEnv, &Loc, &str) -> ()>(&self, report: F, target: &FunctionTarget, initial: Function){
        let mut stack = Vec::<Function>::new();
        stack.push(initial.clone());
        self.traverse_call_graph(&report, target, &initial, &mut stack);
    }
    fn traverse_call_graph<F: Fn(&GlobalEnv, &Loc, &str) -> ()>(&self, report: &F, target: &FunctionTarget, initial: &Function, stack: &mut Vec<Function>){
        if let Some(top) = stack.last(){
            if let Some(calls) = self.functions.get(top){
                for call in calls.iter(){
                    if let Some(index) = stack.iter().position(|x| x == call){
                        let env = target.global_env();
                        let cycle = Self::report_infinite_recursion(stack, call, index, env);
                        report(env, &target.get_loc(), &format!("Unconditional infinite recursion detected: {}", cycle));
                        continue;
                    }
                    stack.push(call.clone());
                    self.traverse_call_graph(report, target, initial, stack);
                    stack.pop();
                }
            }
        }
    }
    fn report_infinite_recursion(stack: &Vec<Function>, f: &Function, position: usize, env: &GlobalEnv) -> String{
        let mut ret = String::new();
        for i in stack.iter().skip(position){
            ret += &i.get_name(env);
            ret += " -> ";
        }
        ret += &f.get_name(env);
        ret
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
enum SegmentId{
    EntryPoint,
    Label(move_stackless_bytecode::stackless_bytecode::Label),
}

#[derive(Clone, Debug)]
enum Terminator{
    Next(SegmentId),
    Jump(SegmentId),
    Branch(SegmentId, SegmentId),
    Return,
}

#[derive(Clone, Debug)]
struct Segment{
    start: usize,
    length: usize,
    term: Terminator,
}

impl Segment {
    fn get_calls(&self, code: &[Bytecode]) -> HashSet<Function>{
        let mut ret = HashSet::new();
        for instr in code.iter().skip(self.start).take(self.length){
            if let Bytecode::Call(_, _, Operation::Function(m, f, _), _, _) = instr{
                ret.insert(Function { module: *m, function: *f });
            }
        }
        ret
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct Function{
    pub module: ModuleId,
    pub function: FunId,
}

impl Function {
    fn get_name(&self, env: &GlobalEnv) -> String{
        self.module.qualified(self.function).get_name_for_display(env).to_string()
    }
}

#[derive(Clone)]
struct ProgramRoute{
    pub current_id: SegmentId,
    pub calls: HashSet<Function>,
    pub visited: HashMap<SegmentId, usize>,
}

impl ProgramRoute {
    fn new() -> ProgramRoute{
        ProgramRoute {
            current_id: SegmentId::EntryPoint,
            calls: HashSet::new(),
            visited: HashMap::new(),
        }
    }
}

struct Program{
    code: Vec<Bytecode>,
    segment_map: HashMap<SegmentId, Segment>,
}

impl Program{
    pub fn new(code: &[Bytecode]) -> Program{
        let mut segmenter = Segmenter::new();
        segmenter.process(code);
        Program {
            code: code.iter().cloned().collect::<Vec<_>>(),
            segment_map: segmenter.get(),
        }
    }
    fn process_unconditional_calls_by_segment(&self) -> HashMap<SegmentId, HashSet<Function>>{
        let mut ret = HashMap::new();
        for (id, segment) in self.segment_map.iter(){
            ret.insert(id.clone(), segment.get_calls(&self.code));
        }
        ret
    }
    pub fn get_unconditional_calls(&self) -> HashSet<Function>{
        let unconditional_calls = self.process_unconditional_calls_by_segment();

        let mut ret: Option<HashSet<Function>> = None;

        let mut stack = Vec::<ProgramRoute>::new();
        stack.push(ProgramRoute::new());
        while let Some(mut route) = stack.pop(){
            let id = route.current_id;
            
            if let Some(visits) = route.visited.get(&id){
                if *visits > 1{
                    continue;
                }
                route.visited.insert(id.clone(), *visits + 1);
            }else{
                route.visited.insert(id.clone(), 1);
                if let Some(calls) = unconditional_calls.get(&id){
                    route.calls = route.calls.union(calls).cloned().collect::<HashSet<_>>();
                }
            };
            
            if let Some(segment) = self.segment_map.get(&id){
                match segment.term.clone(){
                    Terminator::Next(segment_id) | Terminator::Jump(segment_id) => {
                        route.current_id = segment_id;
                        stack.push(route);
                    },
                    Terminator::Branch(left, right) => {
                        route.current_id = left;
                        stack.push(route.clone());
                        route.current_id = right;
                        stack.push(route);
                    },
                    Terminator::Return => {
                        if let Some(set) = ret{
                            ret = Some(set.intersection(&route.calls).cloned().collect::<HashSet<_>>());
                        }else{
                            ret = Some(route.calls);
                        }
                    },
                }
            }
        }

        ret.unwrap_or_else(|| HashSet::<Function>::new())
    }
    #[allow(dead_code)]
    fn print(&self){
        eprintln!("Program{{");
        for (k, v) in self.segment_map.iter(){
            eprintln!("    [{:?}] = {:?},", k, v);
        }
        eprintln!("}}");
    }
}

struct Segmenter{
    result: HashMap<SegmentId, Segment>,
    dead: bool,
    current: usize,
    current_label: Option<Label>,
    i: usize,
}

impl Segmenter {
    pub fn new() -> Segmenter{
        Segmenter{
            result: HashMap::new(),
            dead: false,
            current: 0,
            current_label: None,
            i: 0,
        }
    }
    pub fn process(&mut self, code: &[Bytecode]){
        for instr in code.iter(){
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
                    self.push(Terminator::Branch(SegmentId::Label(*first), SegmentId::Label(*second)));
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
    fn push(&mut self, term: Terminator){
        if self.dead{
            return;
        }

        let segment = Segment {
            start: self.current,
            length: self.i - self.current,
            term
        };

        let id = match self.current_label{
            None => SegmentId::EntryPoint,
            Some(x) => SegmentId::Label(x),
        };
        self.result.insert(id, segment);
    }
    pub fn get(self) -> HashMap<SegmentId, Segment>{
        self.result
    }
}

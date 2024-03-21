// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::core::token_tree::{NestKind, NestKind_, TokenTree};
use crate::syntax_fmt::expr_fmt;
use crate::tools::syntax::parse_file_string;
use crate::tools::utils::FileLineMappingOneFile;
use commentfmt::Config;
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::Definition;
use move_compiler::parser::ast::*;
use move_compiler::parser::lexer::{Lexer, Tok};
use move_compiler::shared::CompilationEnv;
use move_compiler::Flags;
use move_ir_types::location::*;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct FunExtractor {
    pub loc_vec: Vec<Loc>,
    pub ret_ty_loc_vec: Vec<Loc>,
    pub body_loc_vec: Vec<Loc>,
    pub loc_line_vec: Vec<(u32, u32)>,
    pub line_mapping: FileLineMappingOneFile,
    pub belong_module_for_fn: Vec<u32>,
    pub cur_module_id: u32,
}

impl FunExtractor {
    pub fn new(fmt_buffer: String) -> Self {
        let mut this_fun_extractor = Self {
            loc_vec: vec![],
            ret_ty_loc_vec: vec![],
            body_loc_vec: vec![],
            loc_line_vec: vec![],
            line_mapping: FileLineMappingOneFile::default(),
            belong_module_for_fn: vec![],
            cur_module_id: 0,
        };

        this_fun_extractor.line_mapping.update(&fmt_buffer);
        let attrs: BTreeSet<String> = BTreeSet::new();
        let mut env = CompilationEnv::new(Flags::testing(), attrs);
        let filehash = FileHash::empty();
        let (defs, _) = parse_file_string(&mut env, filehash, &fmt_buffer).unwrap();

        for d in defs.iter() {
            this_fun_extractor.cur_module_id += 1;
            this_fun_extractor.collect_definition(d);
        }

        this_fun_extractor
    }

    fn collect_function(&mut self, d: &Function) {
        match &d.body.value {
            FunctionBody_::Defined(..) => {
                let start_line = self
                    .line_mapping
                    .translate(d.loc.start(), d.loc.start())
                    .unwrap()
                    .start
                    .line;
                let end_line = self
                    .line_mapping
                    .translate(d.loc.end(), d.loc.end())
                    .unwrap()
                    .start
                    .line;
                self.loc_vec.push(d.loc);
                self.ret_ty_loc_vec.push(d.signature.return_type.loc);
                self.body_loc_vec.push(d.body.loc);
                self.loc_line_vec.push((start_line, end_line));
                self.belong_module_for_fn.push(self.cur_module_id);
            }
            FunctionBody_::Native => {}
        }
    }

    fn collect_module(&mut self, d: &ModuleDefinition) {
        for m in d.members.iter() {
            if let ModuleMember::Function(x) = &m {
                self.collect_function(x)
            }
        }
    }

    fn collect_script(&mut self, d: &Script) {
        self.collect_function(&d.function);
    }

    fn collect_definition(&mut self, d: &Definition) {
        match d {
            Definition::Module(x) => self.collect_module(x),
            Definition::Address(x) => {
                for x in x.modules.iter() {
                    self.collect_module(x);
                }
            }
            Definition::Script(x) => self.collect_script(x),
        }
    }
}

fn get_nth_line(s: &str, n: usize) -> Option<&str> {
    s.lines().nth(n)
}

fn get_space_cnt_before_line_str(s: &str) -> usize {
    let mut result = 0;
    let trimed_header_prefix = s.trim_start();
    if !trimed_header_prefix.is_empty() {
        if let Some(indent) = s.find(trimed_header_prefix) {
            result = indent;
        }
    }
    result
}

impl FunExtractor {
    pub(crate) fn is_generic_ty_in_fun_header(&self, kind: &NestKind) -> bool {
        let loc_vec = &self.loc_vec;
        let body_loc_vec = &self.body_loc_vec;

        let len = loc_vec.len();
        let mut left = 0;
        let mut right = len;

        while left < right {
            if kind.end_pos < loc_vec[left].start() || kind.start_pos > loc_vec[right - 1].end() {
                return false;
            }

            let mid = left + (right - left) / 2;
            let mid_loc = loc_vec[mid];
            let mid_body_loc = body_loc_vec[mid];

            if mid_loc.start() < kind.start_pos && kind.end_pos < mid_body_loc.start() {
                return true;
            } else if mid_loc.start() < kind.start_pos {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        false
    }
}

pub(crate) fn fun_header_specifier_fmt(specifier: &str, indent_str: &str) -> String {
    // tracing::debug!("fun_specifier_str = {:?}", specifier);

    let mut fun_specifiers_code = vec![];
    let mut lexer = Lexer::new(specifier, FileHash::empty());
    lexer.advance().unwrap();
    while lexer.peek() != Tok::EOF {
        fun_specifiers_code.push((
            lexer.start_loc() as u32,
            (lexer.start_loc() + lexer.content().len()) as u32,
            lexer.content().to_string(),
        ));
        lexer.advance().unwrap();
    }

    // let tokens: Vec<&str> = specifier.split(' ').collect();
    let mut fun_specifiers = vec![];
    for token in specifier.split_whitespace() {
        fun_specifiers.push(token);
    }

    let mut fun_specifier_fmted_str = "".to_string();
    let mut found_specifier = false;
    let mut first_specifier_idx = 0;

    let mut current_specifier_idx = 0;
    let mut last_substr_len = 0;
    for i in 0..fun_specifiers.len() {
        if i < current_specifier_idx {
            continue;
        }
        let specifier_set = fun_specifiers[i];

        let mut parse_access_specifier_list = |last_substr_len: &mut usize,
                                               fun_specifiers_code: &mut Vec<(
            u32,
            u32,
            String,
        )>| {
            let mut chain: Vec<_> = vec![];
            if i + 1 == fun_specifiers.len() {
                return chain;
            }
            let mut old_last_substr_len = *last_substr_len;
            for (j, item_j) in fun_specifiers.iter().enumerate().skip(i + 1) {
                let mut this_token_is_comment = true;
                let iter_specifier = &specifier[*last_substr_len..];
                if let Some(idx) = iter_specifier.find(item_j) {
                    // if this token's pos not comment
                    for token in &mut *fun_specifiers_code {
                        if token.0 == (idx + *last_substr_len) as u32 {
                            this_token_is_comment = false;
                            break;
                        }
                    }
                    old_last_substr_len = *last_substr_len;
                    *last_substr_len = *last_substr_len + idx + item_j.len();
                }

                if this_token_is_comment {
                    // tracing::debug!("intern> this token is comment -- {}",  item_j);
                    chain.push(item_j.to_string());
                    continue;
                }

                if matches!(
                    *item_j,
                    "acquires" | "reads" | "writes" | "pure" | "!acquires" | "!reads" | "!writes"
                ) {
                    current_specifier_idx = j;
                    *last_substr_len = old_last_substr_len;
                    break;
                } else {
                    let judge_new_line = &specifier[old_last_substr_len..*last_substr_len];
                    // tracing::debug!("judge_new_line = {:?}", judge_new_line);
                    if judge_new_line.contains('\n') {
                        chain.push("\n".to_string());
                        let tmp_indent_str = " ".to_string().repeat(
                            indent_str.to_owned().chars().filter(|c| *c == ' ').count() - 2,
                        );
                        chain.push(tmp_indent_str.clone());
                    }

                    chain.push(item_j.to_string());
                }
            }
            // tracing::debug!("intern> chain[{:?}] -- {:?}", i, chain);
            chain
        };

        let mut this_token_is_comment = true;
        let iter_specifier = &specifier[last_substr_len..];
        if let Some(idx) = iter_specifier.find(specifier_set) {
            // if this token's pos not comment
            for token_idx in 0..fun_specifiers_code.len() {
                let token = &fun_specifiers_code[token_idx];
                // tracing::debug!("iter_specifier = {}, specifier_set = {}", iter_specifier, specifier_set);
                // tracing::debug!("token.0 = {}, idx = {}, last_substr_len = {}", token.0, idx, last_substr_len);
                if token.0 == (idx + last_substr_len) as u32 {
                    this_token_is_comment = false;
                    // tracing::debug!("token.0 = {} === idx + last_substr_len = {}", token.0, idx + last_substr_len);
                    fun_specifiers_code.remove(token_idx);
                    break;
                }
            }
            last_substr_len = last_substr_len + idx + specifier_set.len();
        }

        if this_token_is_comment {
            // tracing::debug!("extern> this token is comment -- {}",  specifier_set);
            continue;
        }

        if matches!(
            specifier_set,
            "acquires" | "reads" | "writes" | "pure" | "!acquires" | "!reads" | "!writes"
        ) {
            if !found_specifier {
                first_specifier_idx = last_substr_len - specifier_set.len();
                found_specifier = true;
            }

            fun_specifier_fmted_str.push('\n');
            fun_specifier_fmted_str.push_str(indent_str);
            fun_specifier_fmted_str.push_str(specifier_set);
            if specifier_set != "pure" {
                fun_specifier_fmted_str.push(' ');
                fun_specifier_fmted_str.push_str(
                    &(parse_access_specifier_list(&mut last_substr_len, &mut fun_specifiers_code)
                        .join(" ")),
                );
            }
        }

        // tracing::debug!("<< for loop end, last_substr_len = {}, specifier.len = {}", last_substr_len, specifier.len());
        if last_substr_len == specifier.len() {
            break;
        }
    }

    let mut ret_str = specifier[0..first_specifier_idx].to_string();
    if found_specifier {
        ret_str.push_str(fun_specifier_fmted_str.as_str());
        ret_str.push(' ');
        // tracing::debug!("fun_specifier_fmted_str = --------------{}", ret_str);
    } else {
        ret_str = specifier.to_string();
    }
    ret_str
}

pub(crate) fn process_block_comment_before_fun_header(
    fmt_buffer: String,
    config: Config,
) -> String {
    let buf = fmt_buffer.clone();
    let mut result = fmt_buffer.clone();
    let fun_extractor = FunExtractor::new(fmt_buffer.clone());
    let mut insert_char_nums = 0;
    for (fun_idx, (fun_start_line, _)) in fun_extractor.loc_line_vec.iter().enumerate() {
        let fun_header_str =
            get_nth_line(buf.as_str(), *fun_start_line as usize).unwrap_or_default();
        let mut lexer = Lexer::new(fun_header_str, FileHash::empty());
        lexer.advance().unwrap();
        if lexer.peek() != Tok::EOF && !fun_header_str[0..lexer.start_loc()].trim_start().is_empty()
        {
            let mut insert_str = "\n".to_string();
            insert_str.push_str(" ".to_string().repeat(config.indent_size()).as_str());
            result.insert_str(
                fun_extractor.loc_vec[fun_idx].start() as usize + insert_char_nums,
                &insert_str,
            );
            insert_char_nums += insert_str.len();
        }
    }

    result
}

pub(crate) fn process_fun_header_too_long(fmt_buffer: String, config: Config) -> String {
    let buf = fmt_buffer.clone();
    let mut result = fmt_buffer.clone();
    let fun_extractor = FunExtractor::new(fmt_buffer.clone());
    let mut insert_char_nums = 0;
    let mut fun_idx = 0;
    for fun_loc in fun_extractor.loc_vec.iter() {
        let ret_ty_loc = fun_extractor.ret_ty_loc_vec[fun_idx];
        if ret_ty_loc.start() < fun_loc.start() {
            // this fun return void
            fun_idx += 1;
            continue;
        }

        let mut fun_name_str = &buf[fun_loc.start() as usize..ret_ty_loc.start() as usize];
        if !fun_name_str
            .chars()
            .filter(|&ch| ch == '\n')
            .collect::<String>()
            .is_empty()
        {
            // if it is multi line
            fun_idx += 1;
            continue;
        }
        let ret_ty_len = (ret_ty_loc.end() - ret_ty_loc.start()) as usize;
        if fun_name_str.len() + ret_ty_len < config.max_width() {
            fun_idx += 1;
            continue;
        }

        let mut insert_loc = ret_ty_loc.end() as usize - fun_loc.start() as usize;
        let mut lexer = Lexer::new(fun_name_str, FileHash::empty());
        lexer.advance().unwrap();
        while lexer.peek() != Tok::EOF {
            if lexer.peek() == Tok::Colon {
                insert_loc = lexer.start_loc();
            }
            lexer.advance().unwrap();
        }
        fun_name_str = &buf[fun_loc.start() as usize..(fun_loc.start() as usize) + insert_loc];
        tracing::debug!("fun_name_str = {}", fun_name_str);
        // there maybe comment bewteen fun_name and ret_ty
        if fun_name_str.len() + ret_ty_len < config.max_width() {
            fun_idx += 1;
            continue;
        }

        let mut line_mapping = FileLineMappingOneFile::default();
        line_mapping.update(&fmt_buffer);
        let start_line = line_mapping
            .translate(fun_loc.start(), fun_loc.start())
            .unwrap()
            .start
            .line;
        let fun_header_str = get_nth_line(buf.as_str(), start_line as usize).unwrap_or_default();
        let trimed_header_prefix = fun_header_str.trim_start();
        if !trimed_header_prefix.is_empty() {
            let mut insert_str = "\n".to_string();
            if let Some(indent) = fun_header_str.find(trimed_header_prefix) {
                insert_str.push_str(
                    " ".to_string()
                        .repeat(indent + config.indent_size())
                        .as_str(),
                );
            }
            result.insert_str(
                fun_loc.start() as usize + insert_char_nums + insert_loc,
                &insert_str,
            );
            insert_char_nums += insert_str.len();
        }
        fun_idx += 1;
    }
    result
}

pub(crate) fn process_fun_ret_ty(fmt_buffer: String, config: Config) -> String {
    // process this case:
    // fun fun_name()
    // : u64 {}
    let buf = fmt_buffer.clone();
    let mut result = fmt_buffer.clone();
    let fun_extractor = FunExtractor::new(fmt_buffer.clone());
    let mut insert_char_nums = 0;
    let mut fun_idx = 0;
    for fun_loc in fun_extractor.loc_vec.iter() {
        let ret_ty_loc = fun_extractor.ret_ty_loc_vec[fun_idx];
        if ret_ty_loc.start() < fun_loc.start() {
            // this fun return void
            fun_idx += 1;
            continue;
        }

        let fun_name_str = &buf[fun_loc.start() as usize..ret_ty_loc.start() as usize];
        if fun_name_str.lines().count() > 1 {
            let fun_header_str = buf
                .lines()
                .nth(fun_extractor.loc_line_vec[fun_idx].0 as usize)
                .unwrap_or_default();
            let ret_ty_str = fun_name_str.lines().last().unwrap_or_default();
            let mut lexer = Lexer::new(ret_ty_str, FileHash::empty());
            lexer.advance().unwrap();
            if lexer.peek() != Tok::Colon {
                continue;
            }

            // tracing::debug!("fun_header_str = {:?}, ret_ty_str = {:?}", fun_header_str, ret_ty_str);
            let indent1 = get_space_cnt_before_line_str(fun_header_str);
            let indent2 = get_space_cnt_before_line_str(ret_ty_str);
            // tracing::debug!("indent1 = {}, indent2 = {}", indent1, indent2);
            if indent1 == indent2 {
                // tracing::debug!("fun_header_str = \n{:?}", &buf[0..(ret_ty_loc.start() as usize - ret_ty_str.len())]);
                result.insert_str(
                    ret_ty_loc.start() as usize - ret_ty_str.len() + insert_char_nums,
                    " ".to_string().repeat(config.indent_size()).as_str(),
                );
                insert_char_nums += config.indent_size();
            }
        }
    }
    // tracing::debug!("result = \n{}", result);
    result
}

pub(crate) fn process_fun_annotation(kind: NestKind, elements: Vec<TokenTree>) -> String {
    fn process_simple_token(token: &TokenTree, next_token: Option<&TokenTree>) -> String {
        let mut fmt_result_str = "".to_string();
        fmt_result_str.push_str(token.simple_str().unwrap_or_default());
        if expr_fmt::need_space(token, next_token) {
            fmt_result_str.push(' ');
        }
        fmt_result_str
    }

    fn process_nested_token(nested_token_tree: &TokenTree) -> String {
        let mut fmt_result_str = "".to_string();
        if let TokenTree::Nested {
            elements,
            kind,
            note: _,
        } = nested_token_tree
        {
            fmt_result_str.push_str(kind.start_token_tree().simple_str().unwrap_or_default());
            let mut internal_token_idx = 0;
            while internal_token_idx < elements.len() {
                let t = elements.get(internal_token_idx).unwrap();
                let next_t = elements.get(internal_token_idx + 1);
                fmt_result_str.push_str(&process_token_trees(t, next_t));
                internal_token_idx += 1;
            }
            fmt_result_str.push_str(kind.end_token_tree().simple_str().unwrap_or_default());
        }
        fmt_result_str
    }

    fn process_token_trees(token: &TokenTree, next_token: Option<&TokenTree>) -> String {
        match token {
            TokenTree::Nested {
                elements: _,
                kind: _,
                note: _,
            } => process_nested_token(token),
            TokenTree::SimpleToken {
                content: _,
                pos: _,
                tok: _,
                note: _,
            } => process_simple_token(token, next_token),
        }
    }

    if NestKind_::Bracket == kind.kind {
        let fmt_result_str = process_nested_token(&TokenTree::Nested {
            elements,
            kind,
            note: None,
        });
        tracing::debug!("fmt_result_str = {}", fmt_result_str);
        return fmt_result_str;
    }
    "".to_string()
}

pub fn fmt_fun(fmt_buffer: String, config: Config) -> String {
    let mut result = process_block_comment_before_fun_header(fmt_buffer, config.clone());
    result = process_fun_header_too_long(result, config.clone());
    result = process_fun_ret_ty(result, config.clone());
    result
}

#[test]
fn test_rewrite_fun_header_1() {
    fun_header_specifier_fmt("acquires *(make_up_address(x))", &"    ".to_string());
    fun_header_specifier_fmt("!reads *(0x42), *(0x43)", &"    ".to_string());
    fun_header_specifier_fmt(": u32 !reads *(0x42), *(0x43)", &"    ".to_string());
    fun_header_specifier_fmt(": /*(bool, bool)*/ (bool, bool) ", &"    ".to_string());
}

#[test]
fn test_rewrite_fun_header_2() {
    fun_header_specifier_fmt(
        ": u64 /* acquires comment1 */ acquires SomeStruct ",
        &"    ".to_string(),
    );
    fun_header_specifier_fmt(
        ": u64 acquires SomeStruct/* acquires comment2 */ ",
        &"    ".to_string(),
    );
    fun_header_specifier_fmt(": u64 /* acquires comment3 */ acquires /* acquires comment4 */ SomeStruct /* acquires comment5 */", 
        &"    ".to_string());
    fun_header_specifier_fmt(
        "acquires R reads R writes T, S reads G<u64> ",
        &"    ".to_string(),
    );
    fun_header_specifier_fmt("fun f11() !reads *(0x42) ", &"    ".to_string());
}

#[test]
fn test_rewrite_fun_header_3() {
    fun_header_specifier_fmt(
        "
        // comment1
        econia: &signer)
        acquires // acquires comment2
        IncentiveParameters 
    ",
        &"    ".to_string(),
    );
}

#[test]
fn test_process_block_comment_before_fun_header_1() {
    process_block_comment_before_fun_header(
        "
        module TestFunFormat {
        
            struct SomeOtherStruct has drop {
                some_field: u64,
            } 
            /* BlockComment1 */ public fun multi_arg(p1: u64, p2: u64): u64 {
                p1 + p2
            }
            // test two fun Close together without any blank lines, and here is a InlineComment
            /* BlockComment2 */ public fun multi_arg22(p1: u64, p2: u64): u64 {
                p1 + p2
            } 
            /* BlockComment3 */ /* BlockComment4 */ fun multi_arg22(p1: u64, p2: u64): u64 {
                p1 + p2
            }
        }
        "
        .to_string(),
        Config::default(),
    );
}

#[test]
fn test_process_fun_header_too_long1() {
    let ret_str = process_fun_header_too_long(
"
module TestFunFormat {
    fun test_long_fun_name_lllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll(v: u64): SomeOtherStruct {}

    // xxxx
    fun test_long_fun_name_lllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll(v: u64): SomeOtherStruct {}
}
".to_string(), Config::default());

    tracing::debug!("fun_specifier_fmted_str = --------------{}", ret_str);
}

#[test]
fn test_process_fun_header_too_long2() {
    let ret_str = process_fun_header_too_long(
        "
module 0x42::LambdaTest1 {
    // Public inline function
    public inline fun inline_mul(a: u64, // Input parameter a
        b: u64) // Input parameter b
    : u64 { // Returns a u64 value
        // Multiply a and b
        a * b
    }
}
"
        .to_string(),
        Config::default(),
    );

    tracing::debug!("fun_specifier_fmted_str = --------------{}", ret_str);
}

#[test]
fn test_process_fun_ret_ty() {
    process_fun_ret_ty(
        "
module 0x42::LambdaTest1 {  
    /** Public inline function */  
    public inline fun inline_mul(/** Input parameter a */ a: u64,   
                                 /** Input parameter b */ b: u64)   
    /** Returns a u64 value */ : u64 {  
        /** Multiply a and b */  
        a * b  
    }  
}
"
        .to_string(),
        Config::default(),
    );
}

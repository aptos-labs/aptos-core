// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tools::syntax::parse_file_string;
use crate::tools::utils::FileLineMappingOneFile;
use commentfmt::Config;
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::Definition;
use move_compiler::parser::ast::*;
use move_compiler::parser::lexer::{Lexer, Tok};
use move_compiler::shared::{CompilationEnv, Identifier};
use move_compiler::Flags;
use move_ir_types::location::*;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct SpecExtractor {
    pub fn_loc_vec: Vec<Loc>,
    pub fn_ret_ty_loc_vec: Vec<Loc>,
    pub fn_body_loc_vec: Vec<Loc>,
    pub fn_loc_line_vec: Vec<(u32, u32)>,

    pub stct_loc_vec: Vec<Loc>,
    pub stct_loc_line_vec: Vec<(u32, u32)>,

    pub spec_fn_loc_vec: Vec<Loc>,
    pub spec_fn_name_loc_vec: Vec<Loc>,
    pub spec_fn_para_loc_vec: Vec<Loc>,
    pub spec_fn_ret_ty_loc_vec: Vec<Loc>,
    pub spec_fn_body_loc_vec: Vec<Loc>,
    pub spec_fn_loc_line_vec: Vec<(u32, u32)>,

    pub blk_loc_vec: Vec<Loc>,
    pub line_mapping: FileLineMappingOneFile,
}

impl SpecExtractor {
    pub fn new(fmt_buffer: String) -> Self {
        let mut spec_extractor = Self {
            fn_loc_vec: vec![],
            fn_ret_ty_loc_vec: vec![],
            fn_body_loc_vec: vec![],
            fn_loc_line_vec: vec![],

            stct_loc_vec: vec![],
            stct_loc_line_vec: vec![],

            spec_fn_loc_vec: vec![],
            spec_fn_name_loc_vec: vec![],
            spec_fn_para_loc_vec: vec![],
            spec_fn_ret_ty_loc_vec: vec![],
            spec_fn_body_loc_vec: vec![],
            spec_fn_loc_line_vec: vec![],

            blk_loc_vec: vec![],
            line_mapping: FileLineMappingOneFile::default(),
        };

        spec_extractor.line_mapping.update(&fmt_buffer);
        let attrs: BTreeSet<String> = BTreeSet::new();
        let mut env = CompilationEnv::new(Flags::testing(), attrs);
        let filehash = FileHash::empty();
        let (defs, _) = parse_file_string(&mut env, filehash, &fmt_buffer).unwrap();

        for d in defs.iter() {
            spec_extractor.collect_definition(d);
        }
        spec_extractor
    }

    fn collect_struct(&mut self, s: &StructDefinition) {
        self.stct_loc_vec.push(s.loc);
        self.blk_loc_vec.push(s.loc);
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
                self.fn_loc_vec.push(d.loc);
                self.fn_ret_ty_loc_vec.push(d.signature.return_type.loc);
                self.fn_body_loc_vec.push(d.body.loc);
                self.fn_loc_line_vec.push((start_line, end_line));
                self.blk_loc_vec.push(d.loc);
            }
            FunctionBody_::Native => {}
        }
    }

    fn collect_spec(&mut self, spec_block: &SpecBlock) {
        // tracing::debug!("collect_spec spec_block = {:?}", spec_block);
        self.blk_loc_vec.push(spec_block.loc);

        if let SpecBlockTarget_::Member(member_name, Some(signature)) =
            &spec_block.value.target.value
        {
            let start_line = self
                .line_mapping
                .translate(
                    spec_block.value.target.loc.start(),
                    spec_block.value.target.loc.start(),
                )
                .unwrap()
                .start
                .line;
            let end_line = self
                .line_mapping
                .translate(
                    spec_block.value.target.loc.end(),
                    spec_block.value.target.loc.end(),
                )
                .unwrap()
                .start
                .line;
            self.spec_fn_loc_vec.push(spec_block.value.target.loc);
            self.spec_fn_name_loc_vec.push(member_name.loc);
            self.spec_fn_para_loc_vec
                .push(if !signature.parameters.is_empty() {
                    signature.parameters[0].0.loc()
                } else {
                    signature.return_type.loc
                });
            self.spec_fn_ret_ty_loc_vec.push(signature.return_type.loc);
            // self.spec_fn_body_loc_vec.push(body.loc);
            self.spec_fn_loc_line_vec.push((start_line, end_line));
        }

        for m in spec_block.value.members.iter() {
            // tracing::debug!("collect_spec spec_block.value.member = {:?}", m);
            if let SpecBlockMember_::Function {
                uninterpreted: _,
                name,
                signature,
                body,
            } = &m.value
            {
                if let FunctionBody_::Defined(..) = &body.value {
                    let start_line = self
                        .line_mapping
                        .translate(m.loc.start(), m.loc.start())
                        .unwrap()
                        .start
                        .line;
                    let end_line = self
                        .line_mapping
                        .translate(m.loc.end(), m.loc.end())
                        .unwrap()
                        .start
                        .line;
                    self.spec_fn_loc_vec.push(m.loc);
                    self.spec_fn_name_loc_vec.push(name.0.loc);
                    self.spec_fn_para_loc_vec
                        .push(if !signature.parameters.is_empty() {
                            signature.parameters[0].0.loc()
                        } else {
                            signature.return_type.loc
                        });
                    self.spec_fn_ret_ty_loc_vec.push(signature.return_type.loc);
                    self.spec_fn_body_loc_vec.push(body.loc);
                    self.spec_fn_loc_line_vec.push((start_line, end_line));
                }
            }
        }
    }

    fn collect_module(&mut self, d: &ModuleDefinition) {
        for m in d.members.iter() {
            match &m {
                ModuleMember::Struct(x) => self.collect_struct(x),
                ModuleMember::Function(x) => self.collect_function(x),
                ModuleMember::Spec(s) => self.collect_spec(s),
                _ => {}
            }
        }
    }

    fn collect_script(&mut self, d: &Script) {
        self.collect_function(&d.function);
        for s in d.specs.iter() {
            self.collect_spec(s);
        }
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

pub fn add_blank_row_in_two_blocks(fmt_buffer: String) -> String {
    let buf = fmt_buffer.clone();
    let mut result = fmt_buffer.clone();
    let spec_extractor = SpecExtractor::new(fmt_buffer.clone());
    let mut insert_char_nums = 0;
    for pre_blk_idx in 0..spec_extractor.blk_loc_vec.len() {
        if pre_blk_idx == spec_extractor.blk_loc_vec.len() - 1 {
            break;
        }
        let next_blk_idx = pre_blk_idx + 1;
        let blk1_end_line = spec_extractor
            .line_mapping
            .translate(
                spec_extractor.blk_loc_vec[pre_blk_idx].end(),
                spec_extractor.blk_loc_vec[pre_blk_idx].end(),
            )
            .unwrap()
            .start
            .line;

        let blk2_start_line = spec_extractor
            .line_mapping
            .translate(
                spec_extractor.blk_loc_vec[next_blk_idx].start(),
                spec_extractor.blk_loc_vec[next_blk_idx].start(),
            )
            .unwrap()
            .start
            .line;

        let is_need_blank_row = {
            if blk1_end_line + 1 == blk2_start_line {
                true
            } else {
                let the_row_after_blk1_end =
                    get_nth_line(buf.as_str(), (blk1_end_line + 1) as usize).unwrap_or_default();
                let trimed_prefix = the_row_after_blk1_end.trim_start();
                if !trimed_prefix.is_empty() {
                    // there are code or comment located in line(blk1_end_line + 1)
                    // tracing::debug!("trimed_prefix = {:?}", trimed_prefix);
                    true
                } else {
                    false
                }
            }
        };
        if is_need_blank_row {
            result.insert(
                spec_extractor.blk_loc_vec[pre_blk_idx].end() as usize + insert_char_nums + 1,
                '\n',
            );
            insert_char_nums += 1;
        }
    }

    // tracing::debug!("result = {}", result);
    result
}

pub fn process_block_comment_before_spec_header(fmt_buffer: String, config: Config) -> String {
    let buf = fmt_buffer.clone();
    let mut result = fmt_buffer.clone();
    let spec_extractor = SpecExtractor::new(fmt_buffer.clone());
    let mut insert_char_nums = 0;
    for (fun_idx, (fun_start_line, _)) in spec_extractor.spec_fn_loc_line_vec.iter().enumerate() {
        // tracing::debug!("fun header is {:?}", );
        let fun_header_str =
            get_nth_line(buf.as_str(), *fun_start_line as usize).unwrap_or_default();
        let filehash = FileHash::empty();
        let mut lexer = Lexer::new(fun_header_str, filehash);
        lexer.advance().unwrap();
        if lexer.peek() != Tok::EOF && !fun_header_str[0..lexer.start_loc()].trim_start().is_empty()
        {
            let mut insert_str = "\n".to_string();
            insert_str.push_str(" ".to_string().repeat(config.indent_size()).as_str());
            result.insert_str(
                spec_extractor.spec_fn_loc_vec[fun_idx].start() as usize + insert_char_nums,
                &insert_str,
            );
            insert_char_nums += insert_str.len();
        }
    }

    result
}

pub fn process_spec_fn_header_too_long(fmt_buffer: String, config: Config) -> String {
    let buf = fmt_buffer.clone();
    let mut result = fmt_buffer.clone();
    let spec_extractor = SpecExtractor::new(fmt_buffer.clone());
    let mut insert_char_nums = 0;
    let mut fun_idx = 0;
    for fun_loc in spec_extractor.spec_fn_loc_vec.iter() {
        tracing::debug!("spec_fun_loc = {:?}", fun_loc);
        let ret_ty_loc = spec_extractor.spec_fn_ret_ty_loc_vec[fun_idx];
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

        let para_start_pos_in_header_line = spec_extractor.spec_fn_para_loc_vec[fun_idx].start()
            as usize
            - fun_loc.start() as usize;
        let mut insert_loc = ret_ty_loc.end() as usize - fun_loc.start() as usize;
        let mut lexer = Lexer::new(fun_name_str, FileHash::empty());
        lexer.advance().unwrap();
        while lexer.peek() != Tok::EOF {
            if lexer.peek() == Tok::Colon {
                insert_loc = lexer.start_loc();
            }
            lexer.advance().unwrap();
        }

        // insert pos is (/*insert here*/para1...)/*or insert here*/ : return_type
        insert_loc = if insert_loc <= para_start_pos_in_header_line {
            insert_loc
        } else {
            para_start_pos_in_header_line
        };
        fun_name_str = &buf[fun_loc.start() as usize..(fun_loc.start() as usize) + insert_loc];
        tracing::debug!("spec_fun_name_str = {}", fun_name_str);
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

pub fn fmt_spec(fmt_buffer: String, config: Config) -> String {
    let mut result = process_block_comment_before_spec_header(fmt_buffer, config.clone());
    result = process_spec_fn_header_too_long(result, config.clone());
    result
}

#[test]
fn test_add_blank_row_in_two_blocks_1() {
    add_blank_row_in_two_blocks(
        "
    module std::ascii {
        struct Char {
            byte: u8,
        }
        spec Char {
            // comment
            invariant is_valid_char(byte); //comment
        }
    }    
    "
        .to_string(),
    );
}

#[test]
fn test_process_spec_fn_header_too_long_1() {
    let result = process_spec_fn_header_too_long("
    /// test_point: fun name too long
    spec aptos_std::big_vector {
        // -----------------
        // Data invariants
        // -----------------
        
        spec singletonlllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll<T: store>(element: T, bucket_size: u64): BigVector<T>{
            ensures length(result) == 1;
            ensures result.bucket_size == bucket_size;
        }
    }   
    "
    .to_string(),
    Config::default()
);

    tracing::debug!("result = {}", result);
}

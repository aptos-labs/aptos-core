// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tools::syntax::parse_file_string;
use crate::tools::utils::FileLineMappingOneFile;
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::Definition;
use move_compiler::parser::ast::*;
use move_compiler::shared::CompilationEnv;
use move_compiler::Flags;
use move_ir_types::location::*;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct BigBlockExtractor {
    pub blk_loc_vec: Vec<Loc>,
    pub line_mapping: FileLineMappingOneFile,
}

impl BigBlockExtractor {
    pub fn new(fmt_buffer: String) -> Self {
        let mut big_block_extractor = Self {
            blk_loc_vec: vec![],
            line_mapping: FileLineMappingOneFile::default(),
        };

        big_block_extractor.line_mapping.update(&fmt_buffer);
        let attrs: BTreeSet<String> = BTreeSet::new();
        let mut env = CompilationEnv::new(Flags::testing(), attrs);
        let (defs, _) = parse_file_string(&mut env, FileHash::empty(), &fmt_buffer).unwrap();

        for d in defs.iter() {
            big_block_extractor.collect_definition(d);
        }
        big_block_extractor
    }

    fn collect_struct(&mut self, s: &StructDefinition) {
        self.blk_loc_vec.push(s.loc);
    }

    fn collect_function(&mut self, d: &Function) {
        match &d.body.value {
            FunctionBody_::Defined(..) => {
                self.blk_loc_vec.push(d.loc);
            }
            FunctionBody_::Native => {}
        }
    }

    fn collect_spec(&mut self, spec_block: &SpecBlock) {
        self.blk_loc_vec.push(spec_block.loc);
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
    let big_block_extractor = BigBlockExtractor::new(fmt_buffer.clone());
    // tracing::debug!("blocks = {:?}", big_block_extractor.blk_loc_vec);
    let mut insert_char_nums = 0;
    for pre_blk_idx in 0..big_block_extractor.blk_loc_vec.len() {
        if pre_blk_idx == big_block_extractor.blk_loc_vec.len() - 1 {
            break;
        }
        let next_blk_idx = pre_blk_idx + 1;
        let blk1_end_line = big_block_extractor
            .line_mapping
            .translate(
                big_block_extractor.blk_loc_vec[pre_blk_idx].end(),
                big_block_extractor.blk_loc_vec[pre_blk_idx].end(),
            )
            .unwrap()
            .start
            .line;

        let blk2_start_line = big_block_extractor
            .line_mapping
            .translate(
                big_block_extractor.blk_loc_vec[next_blk_idx].start(),
                big_block_extractor.blk_loc_vec[next_blk_idx].start(),
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
                let trimed_prefix = the_row_after_blk1_end.trim_start().split(' ');
                if trimed_prefix.count() > 1 || the_row_after_blk1_end.trim_start().len() >= 2 {
                    // there are code or comment located in line(blk1_end_line + 1)
                    // tracing::debug!("trimed_prefix = {:?}", trimed_prefix);
                    // tracing::debug!("blk1_end_line = {:?}, blk2_start_line = {:?}", blk1_end_line, blk2_start_line);
                    true
                } else {
                    false
                }
            }
        };
        if is_need_blank_row {
            let mut insert_pos =
                big_block_extractor.blk_loc_vec[pre_blk_idx].end() as usize + insert_char_nums;
            while result.chars().nth(insert_pos).unwrap_or_default() != '\n' {
                insert_pos += 1;
            }
            result.insert(insert_pos, '\n');
            insert_char_nums += 1;
        }
    }
    result
}

pub fn fmt_big_block(fmt_buffer: String) -> String {
    add_blank_row_in_two_blocks(fmt_buffer)
}

#[test]
fn test_add_blank_row_in_two_blocks_1() {
    let result = add_blank_row_in_two_blocks(
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

    tracing::debug!("result = {}", result);
}

#[test]
fn test_add_blank_row_in_two_blocks_2() {
    let result = add_blank_row_in_two_blocks(
        "
module Test {
    struct SomeOtherStruct1 has drop {
        some_other_field1: u64,
    }

    struct SomeOtherStruct2 has drop {
        some_other_field2: SomeOtherStruct1,
    }

    struct SomeOtherStruct3 has drop {
        some_other_field3: SomeOtherStruct2,
    } //comment
    struct SomeOtherStruct4 has drop {
        some_other_field4: SomeOtherStruct3,
    }

    struct SomeOtherStruct5 has drop {
        some_other_field5: SomeOtherStruct4,
    }

    struct SomeOtherStruct6 has drop {
        some_other_field6: SomeOtherStruct5,
    }
    struct SomeStruct has key, drop, store {
        some_field: SomeOtherStruct6,
    }

    fun acq(addr: address): u64
        acquires SomeStruct {
        let val = borrow_global<SomeStruct>(addr);

        val.some_field.some_other_field6.some_other_field5.some_other_field4.some_other_field3.
        some_other_field2.some_other_field1
    }
}
"
        .to_string(),
    );

    tracing::debug!("result = {}", result);
}

#[test]
fn test_add_blank_row_in_two_blocks_3() {
    let result = add_blank_row_in_two_blocks(
        "
module test_module1 {

    struct TestStruct1 {
        // This is field1 comment
        field1: u64,
        field2: bool,
    }
}

module test_module2 {

    struct TestStruct2 { // This is a comment before struct definition
        field1: u64, // This is a comment for field1
        field2: bool, // This is a comment for field2
    } // This is a comment after struct definition
}

module test_module4 {

    struct TestStruct4<T>{
        // This is a comment before complex field
        field: vector<T>, // This is a comment after complex field
    }
}
"
        .to_string(),
    );

    tracing::debug!("result = {}", result);
}

#[test]
fn test_add_blank_row_in_two_blocks_4() {
    let result = add_blank_row_in_two_blocks(
        "
spec std::string {
    spec internal_check_utf8(v: &vector<u8>): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_internal_check_utf8(v);
    }
    spec internal_is_char_boundary(v: &vector<u8>, i: u64): bool {
        pragma opaque;
        aborts_if[abstract] false;
        ensures[abstract] result == spec_internal_is_char_boundary(v, i);
    }
}
    
"
        .to_string(),
    );

    tracing::debug!("result = {}", result);
}

#[test]
fn test_add_blank_row_in_two_blocks_5() {
    let result = add_blank_row_in_two_blocks(
        "
address 0x1 {
    module M {
        #[test]
        #[expected_failure(vector_error, minor_status = 1, location = Self)]
        fun borrow_out_of_range() {}
        #[test]
        #[expected_failure(abort_code = 26113, location = extensions::table)]
        fun test_destroy_fails() {}
    }
}
    "
        .to_string(),
    );

    tracing::debug!("result = {}", result);
}

// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tools::utils::FileLineMappingOneFile;
use commentfmt::Config;
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::Definition;
use move_compiler::parser::ast::*;
use move_compiler::parser::lexer::{Lexer, Tok};
use move_ir_types::location::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug)]
pub struct LetIfElseBlock {
    pub let_if_else_block_loc_vec: Vec<Loc>,
    pub then_in_let_loc_vec: Vec<Loc>,
    pub else_in_let_loc_vec: Vec<Loc>,

    pub let_if_else_block: Vec<lsp_types::Range>,
    pub if_cond_in_let: Vec<lsp_types::Range>,
    pub then_in_let: Vec<lsp_types::Range>,
    pub else_in_let: Vec<lsp_types::Range>,
}

#[derive(Debug)]
pub struct ComIfElseBlock {
    pub if_else_blk_loc_vec: Vec<Loc>,
    pub then_loc_vec: Vec<Loc>,
    pub else_loc_vec: Vec<Loc>,
}

#[derive(Debug)]
pub enum BranchKind {
    LetIfElse,
    ComIfElse,
}

#[derive(Debug)]
pub struct BranchExtractor {
    pub let_if_else: LetIfElseBlock,
    pub com_if_else: ComIfElseBlock,
    pub cur_kind: BranchKind,
    pub source: String,
    pub line_mapping: FileLineMappingOneFile,
    pub added_new_line_branch: RefCell<HashMap<ByteIndex, usize>>,
}

impl BranchExtractor {
    pub fn new(fmt_buffer: String, cur_kind: BranchKind) -> Self {
        let let_if_else = LetIfElseBlock {
            let_if_else_block_loc_vec: vec![],
            then_in_let_loc_vec: vec![],
            else_in_let_loc_vec: vec![],

            let_if_else_block: vec![],
            if_cond_in_let: vec![],
            then_in_let: vec![],
            else_in_let: vec![],
        };
        let com_if_else = ComIfElseBlock {
            if_else_blk_loc_vec: vec![],
            then_loc_vec: vec![],
            else_loc_vec: vec![],
        };
        let mut this_branch_extractor = Self {
            let_if_else,
            com_if_else,
            source: fmt_buffer.clone(),
            line_mapping: FileLineMappingOneFile::default(),
            cur_kind,
            added_new_line_branch: HashMap::default().into(),
        };

        this_branch_extractor
            .line_mapping
            .update(&fmt_buffer.clone());
        this_branch_extractor
    }

    fn get_loc_range(&self, loc: Loc) -> lsp_types::Range {
        self.line_mapping.translate(loc.start(), loc.end()).unwrap()
    }

    fn collect_expr(&mut self, e: &Exp) {
        if let Exp_::IfElse(c, then_, Some(eles)) = &e.value {
            if let BranchKind::LetIfElse = self.cur_kind {
                self.let_if_else.let_if_else_block_loc_vec.push(e.loc);
                self.let_if_else.then_in_let_loc_vec.push(then_.loc);
                self.let_if_else.else_in_let_loc_vec.push(eles.loc);

                self.let_if_else
                    .let_if_else_block
                    .push(self.get_loc_range(e.loc));
                self.let_if_else
                    .if_cond_in_let
                    .push(self.get_loc_range(c.loc));
                self.let_if_else
                    .then_in_let
                    .push(self.get_loc_range(then_.loc));
                self.let_if_else
                    .else_in_let
                    .push(self.get_loc_range(eles.loc));
            }
        }
        if let Exp_::IfElse(_, then_, eles_opt) = &e.value {
            if let BranchKind::ComIfElse = self.cur_kind {
                self.com_if_else.if_else_blk_loc_vec.push(e.loc);
                self.com_if_else.then_loc_vec.push(then_.loc);
                self.collect_expr(then_.as_ref());
                if let Some(el) = eles_opt {
                    self.com_if_else.else_loc_vec.push(el.loc);
                    self.collect_expr(el.as_ref());
                }
            }
        }
    }

    fn collect_seq_item(&mut self, s: &SequenceItem) {
        if let BranchKind::LetIfElse = self.cur_kind {
            if let SequenceItem_::Bind(_, _, e) = &s.value {
                self.collect_expr(e);
            }
        }
        if let BranchKind::ComIfElse = self.cur_kind {
            if let SequenceItem_::Seq(e) = &s.value {
                self.collect_expr(e);
            }
        }
    }

    fn collect_seq(&mut self, s: &Sequence) {
        for s in s.1.iter() {
            self.collect_seq_item(s);
        }
        if let BranchKind::ComIfElse = self.cur_kind {
            if let Some(t) = s.3.as_ref() {
                self.collect_expr(t);
            }
        }
    }

    fn collect_function(&mut self, d: &Function) {
        match &d.body.value {
            FunctionBody_::Defined(s) => {
                self.collect_seq(s);
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

impl BranchExtractor {
    pub fn preprocess(&mut self, module_defs: Vec<Definition>) {
        for d in module_defs.iter() {
            self.collect_definition(d);
        }
    }

    fn need_new_line_in_then_without_brace(
        &self,
        cur_line: String,
        then_start_pos: ByteIndex,
        config: Config,
    ) -> bool {
        for then_loc in &self.com_if_else.then_loc_vec {
            if then_loc.start() == then_start_pos {
                let has_added = cur_line.len() as u32 + then_loc.end() - then_loc.start()
                    > config.max_width() as u32;

                let new_line_cnt = if self
                    .added_new_line_branch
                    .borrow()
                    .contains_key(&then_loc.end())
                {
                    self.added_new_line_branch.borrow_mut()[&then_loc.end()]
                } else {
                    0
                };
                self.added_new_line_branch
                    .borrow_mut()
                    .insert(then_loc.end(), new_line_cnt + has_added as usize);
                return has_added;
            }
        }
        false
    }

    fn need_new_line_after_else(
        &self,
        cur_line: String,
        else_start_pos: ByteIndex,
        config: Config,
    ) -> bool {
        for (else_loc_idx, else_loc) in self.com_if_else.else_loc_vec.iter().enumerate() {
            if else_loc.start() == else_start_pos {
                let mut has_added = cur_line.len() as u32 + else_loc.end() - else_loc.start()
                    > config.max_width() as u32;
                if !has_added && else_loc_idx + 1 < self.com_if_else.else_loc_vec.len() {
                    has_added = self
                        .get_loc_range(self.com_if_else.else_loc_vec[else_loc_idx + 1])
                        .start
                        .line
                        == self.get_loc_range(*else_loc).end.line;
                }

                let new_line_cnt = if self
                    .added_new_line_branch
                    .borrow()
                    .contains_key(&else_loc.end())
                {
                    self.added_new_line_branch.borrow_mut()[&else_loc.end()]
                } else {
                    0
                };

                tracing::debug!(
                    "need_new_line_after_else --> has_added[{:?}] = {:?}, new_line_cnt = {}",
                    cur_line,
                    has_added,
                    new_line_cnt
                );
                self.added_new_line_branch
                    .borrow_mut()
                    .insert(else_loc.end(), new_line_cnt + has_added as usize);
                return has_added;
            }
        }
        false
    }

    pub fn need_new_line_after_branch(
        &self,
        cur_line: String,
        branch_start_pos: ByteIndex,
        config: Config,
    ) -> bool {
        self.need_new_line_in_then_without_brace(cur_line.clone(), branch_start_pos, config.clone())
            || self.need_new_line_after_else(cur_line.clone(), branch_start_pos, config.clone())
    }

    fn added_new_line_in_then_without_brace(&self, then_end_pos: ByteIndex) -> usize {
        for then_loc in &self.com_if_else.then_loc_vec {
            if then_loc.end() == then_end_pos
                && self
                    .added_new_line_branch
                    .borrow()
                    .contains_key(&then_loc.end())
            {
                return self.added_new_line_branch.borrow_mut()[&then_loc.end()];
            }
        }
        0
    }

    fn added_new_line_after_else(&self, else_end_pos: ByteIndex) -> usize {
        for else_loc in &self.com_if_else.else_loc_vec {
            if else_loc.end() == else_end_pos
                && self
                    .added_new_line_branch
                    .borrow()
                    .contains_key(&else_loc.end())
            {
                return self.added_new_line_branch.borrow_mut()[&else_loc.end()];
            }
        }
        0
    }

    pub fn added_new_line_after_branch(&self, branch_end_pos: ByteIndex) -> usize {
        self.added_new_line_in_then_without_brace(branch_end_pos)
            + self.added_new_line_after_else(branch_end_pos)
    }
}

pub fn split_if_else_in_let_block(fmt_buffer: String, config: Config) -> String {
    tracing::debug!("split_if_else_in_let_block >>");
    use crate::tools::syntax::parse_file_string;
    use move_compiler::shared::CompilationEnv;
    use move_compiler::Flags;
    use std::collections::BTreeSet;

    let mut branch_extractor = BranchExtractor::new(fmt_buffer.clone(), BranchKind::LetIfElse);
    let mut env = CompilationEnv::new(Flags::testing(), BTreeSet::new());
    let (defs, _) = parse_file_string(&mut env, FileHash::empty(), &fmt_buffer).unwrap();
    branch_extractor.preprocess(defs);

    let mut result = "".to_string();
    let fmt_lines = fmt_buffer.lines();

    let process_branch = |range: lsp_types::Range| {
        let mut branch_content = "".to_string();
        let mut indent_str = "".to_string();

        let first_line = fmt_lines
            .clone()
            .nth(range.start.line as usize)
            .unwrap_or_default();
        let header_prefix = &first_line[0..range.start.character as usize];
        let trimed_header_prefix = header_prefix.trim_start();
        if !trimed_header_prefix.is_empty() {
            if let Some(indent) = header_prefix.find(trimed_header_prefix) {
                indent_str.push_str(" ".to_string().repeat(indent).as_str());
            }
            indent_str.push_str(" ".to_string().repeat(config.indent_size()).as_str());
            // increase indent
        }

        for line_idx in range.start.line..range.end.line {
            let this_line = fmt_lines.clone().nth(line_idx as usize).unwrap_or_default();
            if line_idx == range.start.line {
                branch_content.push('\n');
                branch_content.push_str(&indent_str);
                branch_content.push_str(this_line[range.start.character as usize..].trim_start());
            } else {
                if branch_content
                    .lines()
                    .last()
                    .map(|x| x.len())
                    .unwrap_or_default()
                    > config.max_width() - 40
                    || branch_content.lines().last().unwrap().contains("//")
                {
                    branch_content.push('\n');
                    branch_content.push_str(&indent_str);
                } else {
                    branch_content.push(' ');
                }
                branch_content.push_str(this_line.trim_start());
            }
        }
        let end_str = fmt_lines
            .clone()
            .nth(range.end.line as usize)
            .unwrap_or_default();
        if range.start.line == range.end.line {
            branch_content.push('\n');
            branch_content.push_str(&indent_str);
            branch_content.push_str(
                end_str[range.start.character as usize..range.end.character as usize].trim_start(),
            );
        } else {
            if branch_content
                .lines()
                .last()
                .map(|x| x.len())
                .unwrap_or_default()
                > config.max_width() - 40
                || branch_content.lines().last().unwrap().contains("//")
            {
                branch_content.push('\n');
                branch_content.push_str(&indent_str);
            } else {
                branch_content.push(' ');
            }
            branch_content.push_str(end_str[0..range.end.character as usize].trim_start());
        }

        // tracing::debug!("branch_content = {}", branch_content);
        branch_content
    };

    let get_else_pos = |let_if_else_loc: Loc, else_branch_in_let_loc: Loc| {
        let branch_str = &fmt_buffer[0..let_if_else_loc.end() as usize];
        let mut lexer = Lexer::new(branch_str, FileHash::empty());
        lexer.advance().unwrap();
        let mut else_in_let_vec = vec![];
        while lexer.peek() != Tok::EOF {
            if lexer.start_loc() >= else_branch_in_let_loc.start() as usize {
                break;
            }
            if let Tok::Else = lexer.peek() {
                else_in_let_vec
                    .push((lexer.start_loc(), lexer.start_loc() + lexer.content().len()));
            }
            lexer.advance().unwrap();
        }

        let ret_pos = else_in_let_vec.last().unwrap();
        (ret_pos.0, ret_pos.1)
    };

    let mut need_split_idx = vec![];
    for let_if_else_idx in 0..branch_extractor.let_if_else.let_if_else_block.len() {
        let start = branch_extractor.let_if_else.let_if_else_block[let_if_else_idx].start;
        let end = branch_extractor.let_if_else.let_if_else_block[let_if_else_idx].end;
        if end.line == start.line && end.character - start.character < 70 {
            continue;
        }
        let then_str = process_branch(branch_extractor.let_if_else.then_in_let[let_if_else_idx]);
        if then_str.contains('{') && then_str.contains('}') {
            // note: maybe comment has "{" or "}"
            continue;
        }
        need_split_idx.push(let_if_else_idx);
    }

    let mut last_pos = (0, 0);
    for idx in need_split_idx {
        let then_str = process_branch(branch_extractor.let_if_else.then_in_let[idx]);
        let else_str = process_branch(branch_extractor.let_if_else.else_in_let[idx]);
        let if_cond_range = branch_extractor.let_if_else.if_cond_in_let[idx];
        let cond_end_line = fmt_lines
            .clone()
            .nth(if_cond_range.end.line as usize)
            .unwrap_or_default();

        // append line[last_line, if ()]
        // eg:
        /*
        // line_x -- last_line
        // ...
        // line_x_plus_n
        if (...)
            ...
        else
            ...
        */
        for idx in last_pos.0..if_cond_range.end.line as usize {
            result.push_str(&fmt_lines.clone().nth(idx).unwrap_or_default()[last_pos.1..]);
            result.push('\n');
            last_pos = (idx + 1, 0);
        }
        result.push_str(&cond_end_line[0..(if_cond_range.end.character) as usize]);

        // append line[if (), then)
        // eg:
        /*
        if (...) /* maybe there has comment1 */ ...
        /* maybe there has
        comment2 */ else /* maybe there has
        comment3 */
            ...
        */
        if if_cond_range.end.line == branch_extractor.let_if_else.then_in_let[idx].start.line {
            result.push_str(
                &cond_end_line[if_cond_range.end.character as usize
                    ..branch_extractor.let_if_else.then_in_let[idx]
                        .start
                        .character as usize],
            );
        }
        result.push_str(&then_str);

        // there maybe comment before else
        let else_pos = get_else_pos(
            branch_extractor.let_if_else.let_if_else_block_loc_vec[idx],
            branch_extractor.let_if_else.else_in_let_loc_vec[idx],
        );
        result.push_str(
            &fmt_buffer
                [branch_extractor.let_if_else.then_in_let_loc_vec[idx].end() as usize..else_pos.0],
        );

        // append "\n$indent_str$else"
        let mut indent_str = "".to_string();
        let header_prefix = &cond_end_line[0..if_cond_range.end.character as usize];
        let trimed_header_prefix = header_prefix.trim_start();
        if !trimed_header_prefix.is_empty() {
            if let Some(indent) = header_prefix.find(trimed_header_prefix) {
                indent_str.push_str(" ".to_string().repeat(indent).as_str());
            }
        }
        result.push('\n');
        result.push_str(&indent_str);
        result.push_str("else");

        // there maybe comment after else
        result.push_str(
            &fmt_buffer[else_pos.1
                ..branch_extractor.let_if_else.else_in_let_loc_vec[idx].start() as usize],
        );
        // append else branch content
        result.push_str(&else_str);

        last_pos = (
            branch_extractor.let_if_else.else_in_let[idx].end.line as usize,
            branch_extractor.let_if_else.else_in_let[idx].end.character as usize,
        );
    }
    tracing::debug!("split_if_else_in_let_block -- processed need_split_idx");
    tracing::debug!("last_pos = {:?}", last_pos);
    let mut byte_idx = 0;
    for idx in 0..last_pos.0 {
        byte_idx += fmt_lines
            .clone()
            .nth(idx)
            .unwrap_or_default()
            .chars()
            .count()
            + 1;
    }
    byte_idx += fmt_lines.clone().nth(last_pos.0).unwrap_or_default()[..last_pos.1].len();
    result.push_str(&fmt_buffer[byte_idx..]);
    tracing::debug!("split_if_else_in_let_block << done !!!");
    result
}

#[test]
fn test_split_if_else_in_let_block_1() {
    split_if_else_in_let_block("
    script {fun main() {  
        // Initialize variable y with value 100  
        let y: u64 = 100;  
        // If y is less than or equal to 10, increment y by 1, otherwise set y to 10  
        let z = if (y /*condition check*/ <= /*less than or equal to*/ 10) y = /*assignment*/ y + /*increment*/ 1 else y = /*assignment*/ 10;  
    }}
    ".to_string(), Config::default());
}

#[test]
fn test_split_if_else_in_let_block_2() {
    split_if_else_in_let_block(
"
script {
    fun main() {
        // Initialize variable y with value 100
        let y: u64 = 100;
        // If y is less than or equal to 10, increment y by 1, otherwise set y to 10
        let z = if (y /*condition check*/ <= /*less than or equal to*/ 10) y = /*assignment*/ y +
        /*increment*/ 1 else y = /*assignment*/ 10;

        // ----------------------------------
        // Initialize variable y with value 100
        let y: u64 = 100;
        // If y is less than or equal to 10, increment y by 1, otherwise set y to 10
        let z = if (y /*condition check*/ <= /*less than or equal to*/ 10) y = /*assignment*/ y + 2 +
        /*increment*/ 1 else y = /*assignment*/ 10;
    }
}
    ".to_string(), Config::default());
}

#[test]
fn test_split_if_else_in_let_block_3() {
    split_if_else_in_let_block(
        "
script {
    fun main() {
        // Initialize variable y with value 100
        let y: u64 = 100;
        // If y is less than or equal to 10, increment y by 1, otherwise set y to 10
        let z = if (y /*condition check*/ <= /*less than or equal to*/ 10) y = /*assignment*/ y +
        /*incre
        xxxxxxxxxxxx
        ment*/ 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16 +
        17 + 18 + 19 + 20 + 21 + 22 + 23 + 24 + 25 + 26 + 27 + 28 + 29 + 30 + 31 + 32 + 33 +
        34 + 35 /*before else comment*/ else /*after else comment*/ y = /*assignment*/ 10;
    }
}
"
        .to_string(),
        Config::default(),
    );
}

#[test]
fn test_split_if_else_in_let_block_4() {
    split_if_else_in_let_block(
"
script {
    fun main() {  
        let y: u64 = 100; // Define an unsigned 64-bit integer variable y and assign it a value of 100  
        let /*comment*/z/*comment*/ = if/*comment*/ (/*comment*/y <= /*comment*/10/*comment*/) { // If y is less than or equal to 10  
            y = y + 1; // Increment y by 1  
        }/*comment*/ else /*comment*/{  
            y = 10; // Otherwise, set y to 10  
        };  
    }
    }
".to_string(), Config::default());
}

#[test]
fn test_split_if_else_in_let_block_5() {
    split_if_else_in_let_block(
"
script {
    fun main() {  
        let y: u64 = 100; // Define an unsigned 64-bit integer variable y and assign it a value of 100  
        let /*comment*/z/*comment*/ = if/*comment*/ (/*comment*/y <= /*comment*/10/*comment*/) { // If y is less than or equal to 10  
            y = y + 1; // Increment y by 1  
        }/*comment*/ else /*comment*/{  
            y = 10; // Otherwise, set y to 10  
        };  

        // ----------
        let y: u64 = 100; // Define an unsigned 64-bit integer variable y and assign it a value of 100  
        let /*comment*/z/*comment*/ = if/*comment*/ (/*comment*/y <= /*comment*/10/*comment*/) { // If y is less than or equal to 10  
            y = y + 1 + 2; // Increment y by 1  
        }/*comment*/ else /*comment*/{  
            y = 10; // Otherwise, set y to 10  
        };  
    }
    }
".to_string(), Config::default());
}

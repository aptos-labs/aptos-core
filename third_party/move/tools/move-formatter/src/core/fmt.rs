// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::core::token_tree::{
    analyze_token_tree_length, Comment, CommentExtrator, CommentKind, Delimiter, NestKind,
    NestKind_, Note, TokenTree,
};
use crate::syntax_fmt::branch_fmt::{self, BranchExtractor, BranchKind};
use crate::syntax_fmt::fun_fmt::FunExtractor;
use crate::syntax_fmt::{big_block_fmt, expr_fmt, fun_fmt, spec_fmt};
use crate::tools::syntax::{self, parse_file_string};
use crate::tools::utils::{FileLineMappingOneFile, Timer};
use commentfmt::{Config, Verbosity};
use move_command_line_common::files::FileHash;
use move_compiler::diagnostics::Diagnostics;
use move_compiler::parser::lexer::{Lexer, Tok};
use move_compiler::shared::CompilationEnv;
use move_compiler::Flags;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::result::Result::*;

pub enum FormatEnv {
    FormatModule,
    FormatUse,
    FormatStruct,
    FormatExp,
    FormatTuple,
    FormatList,
    FormatLambda,
    FormatFun,
    FormatSpecModule,
    FormatSpecStruct,
    FormatSpecFun,
    FormatDefault,
}
pub struct FormatContext {
    pub content: String,
    pub env: FormatEnv,
    pub cur_module_name: String,
    pub cur_tok: Tok,
}

impl FormatContext {
    pub fn new(content: String, env: FormatEnv) -> Self {
        FormatContext {
            content,
            env,
            cur_module_name: "".to_string(),
            cur_tok: Tok::Module,
        }
    }

    pub fn set_env(&mut self, env: FormatEnv) {
        self.env = env;
    }
}

pub struct SyntaxExtractor {
    pub branch_extractor: BranchExtractor,
    pub fun_extractor: FunExtractor,
}

pub struct Format {
    pub local_cfg: FormatConfig,
    pub global_cfg: Config,
    pub depth: Cell<usize>,
    pub token_tree: Vec<TokenTree>,
    pub comments: Vec<Comment>,
    pub line_mapping: FileLineMappingOneFile,
    pub comments_index: Cell<usize>,
    pub ret: RefCell<String>,
    pub cur_line: Cell<u32>,
    pub format_context: RefCell<FormatContext>,
    pub syntax_extractor: SyntaxExtractor,
}

#[derive(Clone, Default)]
pub struct FormatConfig {
    pub max_with: usize,
    pub indent_size: usize,
}

impl Format {
    fn new(global_cfg: Config, content: &str, format_context: FormatContext) -> Self {
        let ce: CommentExtrator = CommentExtrator::new(content).unwrap();
        let mut line_mapping = FileLineMappingOneFile::default();
        line_mapping.update(content);
        let syntax_extractor = SyntaxExtractor {
            branch_extractor: BranchExtractor::new(content.to_string(), BranchKind::ComIfElse),
            fun_extractor: FunExtractor::new(content.to_string()),
        };
        Self {
            comments_index: Default::default(),
            local_cfg: FormatConfig {
                max_with: global_cfg.max_width(),
                indent_size: global_cfg.indent_size(),
            },
            global_cfg,
            depth: Default::default(),
            token_tree: vec![],
            comments: ce.comments,
            line_mapping,
            ret: Default::default(),
            cur_line: Default::default(),
            format_context: format_context.into(),
            syntax_extractor,
        }
    }

    fn generate_token_tree(&mut self, content: &str) -> Result<String, Diagnostics> {
        // let attrs: BTreeSet<String> = BTreeSet::new();
        let mut env = CompilationEnv::new(Flags::testing(), BTreeSet::new());
        let (defs, _) = parse_file_string(&mut env, FileHash::empty(), content)?;
        let lexer = Lexer::new(content, FileHash::empty());
        let parse = crate::core::token_tree::Parser::new(lexer, &defs);
        self.token_tree = parse.parse_tokens();
        self.syntax_extractor.branch_extractor.preprocess(defs);
        Ok("parse ok".to_string())
    }

    fn post_process(&mut self) {
        tracing::debug!("post_process >> meet Brace");
        self.remove_trailing_whitespaces();
        tracing::debug!("post_process -- fmt_fun");
        *self.ret.borrow_mut() =
            fun_fmt::fmt_fun(self.ret.clone().into_inner(), self.global_cfg.clone());
        tracing::debug!("post_process -- split_if_else_in_let_block");
        *self.ret.borrow_mut() = branch_fmt::split_if_else_in_let_block(
            self.ret.clone().into_inner(),
            self.global_cfg.clone(),
        );

        if self.ret.clone().into_inner().contains("spec ") {
            *self.ret.borrow_mut() =
                spec_fmt::fmt_spec(self.ret.clone().into_inner(), self.global_cfg.clone());
        }
        tracing::debug!("post_process -- fmt_big_block");
        *self.ret.borrow_mut() = big_block_fmt::fmt_big_block(self.ret.clone().into_inner());
        self.remove_trailing_whitespaces();
        tracing::debug!("post_process << done !!!");
    }

    pub fn format_token_trees(mut self) -> String {
        let length = self.token_tree.len();
        let mut index = 0;
        let mut pound_sign = None;
        while index < length {
            let t = self.token_tree.get(index).unwrap();
            if t.is_pound() {
                pound_sign = Some(index);
            }
            let new_line = pound_sign.map(|x| (x + 1) == index).unwrap_or_default();
            self.format_token_trees_internal(t, self.token_tree.get(index + 1), new_line);
            if new_line {
                self.new_line(Some(t.end_pos()));
                pound_sign = None;
            }
            // top level
            match t {
                TokenTree::SimpleToken {
                    content: _,
                    pos: _,
                    tok: _,
                    note: _,
                } => {}
                TokenTree::Nested {
                    elements: _,
                    kind,
                    note: _,
                } => {
                    if kind.kind == NestKind_::Brace {
                        self.new_line(Some(t.end_pos()));
                        self.post_process();
                    }
                }
            }
            index += 1;
        }
        self.add_comments(u32::MAX, "end_of_move_file".to_string());
        self.process_last_empty_line();
        self.ret.into_inner()
    }

    fn need_new_line(
        kind: NestKind_,
        delimiter: Option<Delimiter>,
        _has_colon: bool,
        current: &TokenTree,
        next: Option<&TokenTree>,
    ) -> bool {
        if next.and_then(|x| x.simple_str()) == delimiter.map(|x| x.to_static_str()) {
            return false;
        }

        let b_judge_next_token = if let Some((next_tok, next_content)) = next.map(|x| match x {
            TokenTree::SimpleToken {
                content,
                pos: _,
                tok,
                note: _,
            } => (*tok, content.clone()),
            TokenTree::Nested {
                elements: _,
                kind,
                note: _,
            } => (kind.kind.start_tok(), kind.kind.start_tok().to_string()),
        }) {
            match next_tok {
                Tok::Friend
                | Tok::Const
                | Tok::Fun
                | Tok::While
                | Tok::Use
                | Tok::Struct
                | Tok::Spec
                | Tok::Return
                | Tok::Public
                | Tok::Native
                | Tok::Move
                | Tok::Module
                | Tok::Loop
                | Tok::Let
                | Tok::Invariant
                | Tok::If
                | Tok::Continue
                | Tok::Break
                | Tok::NumSign
                | Tok::Abort => true,
                Tok::Identifier => next_content.as_str() == "entry",
                _ => false,
            }
        } else {
            true
        };

        // special case for `}}`
        if match current {
            TokenTree::SimpleToken {
                content: _,
                pos: _,
                tok: _,
                note: _,
            } => false,
            TokenTree::Nested {
                elements: _,
                kind,
                note: _,
            } => kind.kind == NestKind_::Brace,
        } && kind == NestKind_::Brace
            && b_judge_next_token
        {
            return true;
        }
        false
    }

    fn need_new_line_for_each_token_in_nested(
        kind: &NestKind,
        elements: &[TokenTree],
        delimiter: Option<Delimiter>,
        has_colon: bool,
        index: usize,
        new_line_mode: bool,
    ) -> bool {
        let t = elements.get(index).unwrap();
        let next_t = elements.get(index + 1);
        let d = delimiter.map(|x| x.to_static_str());
        let t_str = t.simple_str();

        let mut new_line = if new_line_mode {
            Self::need_new_line(kind.kind, delimiter, has_colon, t, next_t)
                || (d == t_str && d.is_some())
        } else {
            false
        };

        // comma in fun resource access specifier not change new line
        if d == t_str && d.is_some() {
            if let Some(deli_str) = d {
                if deli_str.contains(',') {
                    let mut idx = index;
                    while idx != 0 {
                        let ele = elements.get(idx).unwrap();
                        idx -= 1;
                        if let Some(key) = ele.simple_str() {
                            if key.contains("fun") {
                                break;
                            }
                        }
                        if ele.simple_str().is_none() {
                            continue;
                        }
                        if matches!(
                            ele.simple_str().unwrap(),
                            "acquires" | "reads" | "writes" | "pure"
                        ) {
                            new_line = false;
                            break;
                        }
                    }
                }
            }
        }

        if let Some((next_tok, next_content)) = next_t.map(|x| match x {
            TokenTree::SimpleToken {
                content,
                pos: _,
                tok,
                note: _,
            } => (*tok, content.clone()),
            TokenTree::Nested {
                elements: _,
                kind,
                note: _,
            } => (kind.kind.start_tok(), kind.kind.start_tok().to_string()),
        }) {
            // ablility not change new line
            if syntax::token_to_ability(next_tok, &next_content).is_some() {
                new_line = false;
            }
        }
        new_line
    }

    fn process_fn_header_before_before_fn_nested(&self) {
        let cur_ret = self.ret.clone().into_inner();
        // tracing::debug!("fun_header = {:?}", &self.format_context.content[(kind.start_pos as usize)..(kind.end_pos as usize)]);
        if let Some(last_fun_idx) = cur_ret.rfind("fun") {
            let fun_header: &str = &cur_ret[last_fun_idx..];
            if let Some(specifier_idx) = fun_header.rfind("fun") {
                let indent_str = " "
                    .to_string()
                    .repeat((self.depth.get() + 1) * self.local_cfg.indent_size);
                let fun_specifier_fmted_str = fun_fmt::fun_header_specifier_fmt(
                    &fun_header[specifier_idx + 1..],
                    &indent_str,
                );

                let ret_copy = &self.ret.clone().into_inner()[0..last_fun_idx + specifier_idx + 1];
                let mut new_ret = ret_copy.to_string();
                new_ret.push_str(fun_specifier_fmted_str.as_str());
                *self.ret.borrow_mut() = new_ret.to_string();
            }
        }
        if self.ret.clone().into_inner().contains("writes") {
            tracing::debug!("self.last_line = {:?}", self.last_line());
        }
    }

    fn get_new_line_mode_begin_nested(
        &self,
        kind: &NestKind,
        elements: &Vec<TokenTree>,
        note: &Option<Note>,
        delimiter: Option<Delimiter>,
    ) -> (bool, Option<bool>) {
        let stct_def = note
            .map(|x| x == Note::StructDefinition)
            .unwrap_or_default();
        let fun_body = note.map(|x| x == Note::FunBody).unwrap_or_default();

        let max_len_when_no_add_line: usize = self.global_cfg.max_width() / 3;
        let nested_token_len = analyze_token_tree_length(elements, self.global_cfg.max_width());

        if fun_body {
            self.process_fn_header_before_before_fn_nested();
        }
        let mut new_line_mode = {
            delimiter
                .map(|x| x == Delimiter::Semicolon)
                .unwrap_or_default()
                || (stct_def && !elements.is_empty())
                || (fun_body && !elements.is_empty())
                || self.get_cur_line_len() + nested_token_len > self.global_cfg.max_width()
        };
        if new_line_mode && kind.kind != NestKind_::Type {
            return (true, None);
        }

        match kind.kind {
            NestKind_::Type => {
                // added in 20240112: if type in fun header, not change new line
                if self
                    .syntax_extractor
                    .fun_extractor
                    .is_generic_ty_in_fun_header(kind)
                {
                    return (false, None);
                }
                new_line_mode = nested_token_len > max_len_when_no_add_line;
            }
            NestKind_::ParentTheses => {
                if self.format_context.borrow().cur_tok == Tok::If {
                    new_line_mode = false;
                } else {
                    new_line_mode =
                        !expr_fmt::judge_simple_paren_expr(kind, elements, self.global_cfg.clone());
                    return (new_line_mode, Some(new_line_mode));
                }
            }
            NestKind_::Bracket => {
                new_line_mode = nested_token_len > max_len_when_no_add_line;
            }
            NestKind_::Lambda => {
                if delimiter.is_none() && nested_token_len <= max_len_when_no_add_line {
                    new_line_mode = false;
                }
            }
            NestKind_::Brace => {
                new_line_mode = self.last_line().contains("module")
                    || nested_token_len > max_len_when_no_add_line;
            }
        }
        (new_line_mode, None)
    }

    fn add_new_line_after_nested_begin(
        &self,
        kind: &NestKind,
        elements: &[TokenTree],
        b_new_line_mode: bool,
    ) {
        if !b_new_line_mode {
            if let NestKind_::Brace = kind.kind {
                if elements.len() == 1 {
                    self.push_str(" ");
                }
            }
            return;
        }

        if !elements.is_empty() {
            let next_token = elements.first().unwrap();
            let mut next_token_start_pos: u32 = 0;
            self.analyzer_token_tree_start_pos_(&mut next_token_start_pos, next_token);
            if self.translate_line(next_token_start_pos) > self.translate_line(kind.start_pos) {
                // let source = self.format_context.content.clone();
                // let start_pos: usize = next_token_start_pos as usize;
                // let (_, next_str) = source.split_at(start_pos);
                // tracing::debug!("after format_token_trees_internal<TokenTree::Nested-start_token_tree> return, next_token = {:?}",
                //     next_str);
                // process line tail comment
                self.process_same_line_comment(kind.start_pos, true);
            } else {
                self.new_line(Some(kind.start_pos));
            }
        } else {
            self.new_line(Some(kind.start_pos));
        }
    }

    fn format_single_token(
        &self,
        nested_token: &TokenTree,
        internal_token_idx: usize,
        pound_sign_new_line: bool,
        new_line: bool,
        pound_sign: &mut Option<usize>,
    ) {
        let TokenTree::Nested {
            elements,
            kind,
            note: _,
        } = nested_token
        else {
            return;
        };
        let token = elements.get(internal_token_idx).unwrap();
        let next_t = elements.get(internal_token_idx + 1);

        if !new_line && token.simple_str().is_some() {
            if let NestKind_::Brace = kind.kind {
                if elements.len() == 1 {
                    self.push_str(" ");
                }
            }
        }
        self.format_token_trees_internal(token, next_t, pound_sign_new_line || new_line);
        if pound_sign_new_line {
            tracing::debug!("in loop<TokenTree::Nested> pound_sign_new_line = true");
            self.new_line(Some(token.end_pos()));
            *pound_sign = None;
            return;
        }

        if new_line {
            let process_tail_comment_of_line = match next_t {
                Some(next_token) => {
                    let mut next_token_start_pos: u32 = 0;
                    self.analyzer_token_tree_start_pos_(&mut next_token_start_pos, next_token);
                    self.translate_line(next_token_start_pos) > self.translate_line(token.end_pos())
                }
                None => true,
            };
            self.process_same_line_comment(token.end_pos(), process_tail_comment_of_line);
        } else if let NestKind_::Brace = kind.kind {
            if elements.len() == 1 {
                self.push_str(" ");
            }
        }
    }

    fn format_each_token_in_nested_elements(
        &self,
        kind: &NestKind,
        elements: &[TokenTree],
        delimiter: Option<Delimiter>,
        has_colon: bool,
        b_new_line_mode: bool,
    ) {
        let nested_token = TokenTree::Nested {
            elements: elements.to_owned(),
            kind: *kind,
            note: None,
        };
        let mut pound_sign = None;
        let len = elements.len();
        let mut internal_token_idx = 0;
        while internal_token_idx < len {
            let pound_sign_new_line = pound_sign
                .map(|x| (x + 1) == internal_token_idx)
                .unwrap_or_default();

            let new_line = Self::need_new_line_for_each_token_in_nested(
                kind,
                elements,
                delimiter,
                has_colon,
                internal_token_idx,
                b_new_line_mode,
            );

            if elements.get(internal_token_idx).unwrap().is_pound() {
                pound_sign = Some(internal_token_idx)
            }

            if Tok::Period == self.format_context.borrow().cur_tok {
                let (continue_dot_cnt, index) =
                    expr_fmt::process_link_access(elements, internal_token_idx + 1);
                if continue_dot_cnt > 3 && index > internal_token_idx {
                    self.inc_depth();
                    let mut is_dot_new_line = true;
                    while internal_token_idx <= index + 1 {
                        self.format_single_token(
                            &nested_token,
                            internal_token_idx,
                            false,
                            is_dot_new_line,
                            &mut pound_sign,
                        );
                        internal_token_idx += 1;
                        is_dot_new_line = !is_dot_new_line;
                    }
                    // tracing::debug!("after processed link access, ret = {}", self.ret.clone().into_inner());
                    // tracing::debug!("after processed link access, internal_token_idx = {}, len = {}", internal_token_idx, len);
                    self.dec_depth();
                    continue;
                }
            }

            self.format_single_token(
                &nested_token,
                internal_token_idx,
                pound_sign_new_line,
                new_line,
                &mut pound_sign,
            );
            internal_token_idx += 1;
        }
    }

    fn format_nested_token(&self, token: &TokenTree, next_token: Option<&TokenTree>) {
        if let TokenTree::Nested {
            elements,
            kind,
            note,
        } = token
        {
            let (delimiter, has_colon) = Self::analyze_token_tree_delimiter(elements);
            let (b_new_line_mode, opt_mode) =
                self.get_new_line_mode_begin_nested(kind, elements, note, delimiter);
            let b_add_indent = !note.map(|x| x == Note::ModuleAddress).unwrap_or_default();
            let nested_token_head = self.format_context.borrow().cur_tok;

            if b_new_line_mode {
                tracing::debug!(
                    "nested_token_head = [{:?}], add a new line",
                    nested_token_head
                );
            }

            if Tok::NumSign == nested_token_head {
                self.push_str(fun_fmt::process_fun_annotation(*kind, elements.to_vec()));
                return;
            }

            // step1 -- format start_token
            self.format_token_trees_internal(&kind.start_token_tree(), None, b_new_line_mode);

            // step2 -- paired effect with step6
            if b_add_indent {
                self.inc_depth();
            }

            // step3
            if b_new_line_mode {
                self.add_new_line_after_nested_begin(kind, elements, b_new_line_mode);
            }

            // step4 -- format elements
            let need_change_line_for_each_item_in_paren = if NestKind_::ParentTheses == kind.kind {
                if opt_mode.is_none() {
                    !expr_fmt::judge_simple_paren_expr(kind, elements, self.global_cfg.clone())
                } else {
                    opt_mode.unwrap_or_default()
                }
            } else {
                b_new_line_mode
            };
            self.format_each_token_in_nested_elements(
                kind,
                elements,
                delimiter,
                has_colon,
                need_change_line_for_each_item_in_paren,
            );

            // step5 -- add_comments which before kind.end_pos
            self.add_comments(
                kind.end_pos,
                kind.end_token_tree()
                    .simple_str()
                    .unwrap_or_default()
                    .to_string(),
            );
            let ret_copy = self.ret.clone().into_inner();
            // may be already add_a_new_line in step5 by add_comments(doc_comment in tail of line)
            *self.ret.borrow_mut() = ret_copy.trim_end().to_string();
            if ret_copy.ends_with(' ') {
                self.push_str(" ");
            }
            let had_rm_added_new_line =
                self.ret.clone().into_inner().lines().count() < ret_copy.lines().count();

            // step6 -- paired effect with step2
            if b_add_indent {
                self.dec_depth();
            }

            // step7
            if b_new_line_mode || had_rm_added_new_line {
                tracing::debug!("end_of_nested_block, b_new_line_mode = true");
                if nested_token_head != Tok::If {
                    self.new_line(Some(kind.end_pos));
                }
            }

            // step8 -- format end_token
            self.format_token_trees_internal(&kind.end_token_tree(), None, false);
            if let TokenTree::SimpleToken {
                content: _,
                pos: _t_pos,
                tok: _t_tok,
                note: _,
            } = kind.end_token_tree()
            {
                if expr_fmt::need_space(token, next_token) {
                    self.push_str(" ");
                }
            }
        }
    }

    fn format_simple_token(
        &self,
        token: &TokenTree,
        next_token: Option<&TokenTree>,
        new_line_after: bool,
    ) {
        if let TokenTree::SimpleToken {
            content,
            pos,
            tok,
            note,
        } = token
        {
            // added in 20240115
            // updated in 20240124
            if Tok::LBrace != *tok
                && self
                    .syntax_extractor
                    .branch_extractor
                    .need_new_line_after_branch(self.last_line(), *pos, self.global_cfg.clone())
            {
                tracing::debug!("need_new_line_after_branch[{:?}], add a new line", content);
                self.inc_depth();
                self.new_line(None);
            }

            // process `else if`
            if *tok == Tok::Else && next_token.is_some() && 
                (next_token.unwrap().simple_str().unwrap_or_default() == "if" || 
                self.syntax_extractor
                    .branch_extractor
                    .is_nested_within_an_outer_else(*pos)) {
                self.new_line(None);
            }

            // add comment(xxx) before current simple_token
            self.add_comments(*pos, content.clone());
            /*
            ** simple1:
            self.translate_line(*pos) = 6
            after processed xxx, self.cur_line.get() = 5;
            self.translate_line(*pos) - self.cur_line.get() == 1
            """
            line5: // comment xxx
            line6: simple_token
            """
            */
            if (self.translate_line(*pos) - self.cur_line.get()) > 1 {
                // There are multiple blank lines between the cur_line and the current code simple_token
                tracing::debug!(
                    "self.translate_line(*pos) = {}, self.cur_line.get() = {}",
                    self.translate_line(*pos),
                    self.cur_line.get()
                );
                tracing::debug!("SimpleToken[{:?}], add a new line", content);
                self.new_line(None);
            }

            // added in 20240115
            // updated in 20240124
            // updated in 20240222: remove condition `if Tok::RBrace != *tok `
            {
                let tok_end_pos = *pos + content.len() as u32;
                let mut nested_branch_depth = self
                    .syntax_extractor
                    .branch_extractor
                    .added_new_line_after_branch(tok_end_pos);

                if nested_branch_depth > 0 {
                    tracing::debug!(
                        "nested_branch_depth[{:?}] = [{:?}]",
                        content,
                        nested_branch_depth
                    );
                }
                while nested_branch_depth > 0 {
                    self.dec_depth();
                    nested_branch_depth -= 1;
                }
            }

            // add blank row between module
            if Tok::Module == *tok {
                // tracing::debug!("SimpleToken[{:?}], cur_module_name = {:?}", content,  self.format_context.borrow_mut().cur_module_name);
                if !self.format_context.borrow_mut().cur_module_name.is_empty()
                    && (self.translate_line(*pos) - self.cur_line.get()) >= 1
                {
                    // tracing::debug!("SimpleToken[{:?}], add blank row between module", content);
                    self.new_line(None);
                }
                self.format_context
                    .borrow_mut()
                    .set_env(FormatEnv::FormatModule);
                self.format_context.borrow_mut().cur_module_name = next_token
                    .unwrap()
                    .simple_str()
                    .unwrap_or_default()
                    .to_string();
                // Note: You can get the token tree of the entire format_context.env here
            }

            if content == "if" {
                self.format_context
                    .borrow_mut()
                    .set_env(FormatEnv::FormatExp);
            }
            self.format_context.borrow_mut().cur_tok = *tok;

            let mut split_line_after_content = false;
            if self.judge_change_new_line_when_over_limits(*tok, *note, next_token) {
                tracing::debug!("last_line = {:?}", self.last_line());
                tracing::debug!(
                    "SimpleToken{:?} too long, add a new line because of split line",
                    content
                );

                if matches!(*tok,
                    | Tok::Equal
                    | Tok::EqualEqual
                    | Tok::EqualEqualGreater
                    | Tok::LessEqualEqualGreater) {
                    self.push_str(content.as_str());
                    split_line_after_content = true;
                }
                self.inc_depth();
                self.new_line(None);
                self.dec_depth();
            }

            if !split_line_after_content {
                self.push_str(content.as_str());
            }

            self.cur_line.set(self.translate_line(*pos));
            if new_line_after {
                return;
            }
            if self.judge_change_new_line_when_over_limits(*tok, *note, next_token) {
                tracing::debug!("last_line = {:?}", self.last_line());
                tracing::debug!(
                    "SimpleToken{:?}, add a new line because of split line",
                    content
                );
                self.inc_depth();
                self.new_line(None);
                self.dec_depth();
                return;
            }
            if expr_fmt::need_space(token, next_token) {
                self.push_str(" ");
            }
        }
    }

    fn format_token_trees_internal(
        &self,
        token: &TokenTree,
        next_token: Option<&TokenTree>,
        new_line_after: bool,
    ) {
        match token {
            TokenTree::Nested {
                elements: _,
                kind: _,
                note: _,
            } => self.format_nested_token(token, next_token),
            TokenTree::SimpleToken {
                content: _,
                pos: _,
                tok: _,
                note: _,
            } => self.format_simple_token(token, next_token, new_line_after),
        }
    }

    fn add_comments(&self, pos: u32, content: String) {
        let mut comment_nums_before_cur_simple_token = 0;
        let mut last_cmt_is_block_cmt = false;
        let mut last_cmt_start_pos = 0;
        for c in &self.comments[self.comments_index.get()..] {
            if c.start_offset > pos {
                break;
            }

            if (self.translate_line(c.start_offset) - self.cur_line.get()) > 1 {
                tracing::debug!("the pos of this comment > current line");
                // 20240318: process case as follows
                // 
                /*
                #[test(econia = @econia, integrator = @user)]

                // comment 
                fun func() {}
                */
                if self.format_context.borrow().cur_tok != Tok::NumSign {
                    self.new_line(None);
                }
            }

            if (self.translate_line(c.start_offset) - self.cur_line.get()) == 1 {
                // if located after nestedToken start, maybe already chanedLine
                let ret_copy = self.ret.clone().into_inner();
                *self.ret.borrow_mut() = ret_copy.trim_end().to_string();
                self.new_line(None);
            }

            // tracing::debug!("-- add_comments: line(c.start_offset) - cur_line = {:?}",
            //     self.translate_line(c.start_offset) - self.cur_line.get());
            // tracing::debug!("c.content.as_str() = {:?}\n", c.content.as_str());
            if self.no_space_or_new_line_for_comment() {
                self.push_str(" ");
            }

            self.push_str(c.format_comment(
                c.comment_kind(),
                self.depth.get() * self.local_cfg.indent_size,
                0,
                &self.global_cfg,
            ));

            match c.comment_kind() {
                CommentKind::DocComment => {
                    // let buffer = self.ret.clone();
                    // let len: usize = c.content.len();
                    // let x: usize = buffer.borrow().len();
                    // if len + 2 < x {
                    //     if let Some(ch) = buffer.clone().borrow().chars().nth(x - len - 2) {
                    //         if !ch.is_ascii_whitespace() {
                    //             // insert black space after '//'
                    //             self.ret.borrow_mut().insert(x - len - 1, ' ');
                    //         }
                    //     }
                    // }
                    self.new_line(None);
                    last_cmt_is_block_cmt = false;
                }
                _ => {
                    let end = c.start_offset + (c.content.len() as u32);
                    let line_start = self.translate_line(c.start_offset);
                    let line_end = self.translate_line(end);

                    if !content.contains(')')
                        && !content.contains(',')
                        && !content.contains(';')
                        && !content.contains('.')
                    {
                        self.push_str(" ");
                    }
                    if line_start != line_end {
                        self.new_line(None);
                    }
                    last_cmt_is_block_cmt = true;
                }
            }
            self.comments_index.set(self.comments_index.get() + 1);
            self.cur_line
                .set(self.translate_line(c.start_offset + (c.content.len() as u32) - 1));
            comment_nums_before_cur_simple_token += 1;
            last_cmt_start_pos = c.start_offset;
            // tracing::debug!("in add_comments for loop: self.cur_line = {:?}\n", self.cur_line);
        }
        if comment_nums_before_cur_simple_token > 0 {
            if last_cmt_is_block_cmt
                && self.translate_line(pos) - self.translate_line(last_cmt_start_pos) == 1
            {
                // process this case:
                // line[i]: /*comment1*/ /*comment2*/
                // line[i+1]: code // located in `pos`
                self.new_line(None);
            }
            tracing::debug!(
                "add_comments[{:?}] before pos[{:?}] = {:?} return <<<<<<<<<\n",
                comment_nums_before_cur_simple_token,
                pos,
                content
            );
        }
    }
}

impl Format {
    fn inc_depth(&self) {
        let old = self.depth.get();
        self.depth.set(old + 1);
    }
    fn dec_depth(&self) {
        let old = self.depth.get();
        if old == 0 {
            eprintln!("old depth is zero, return");
            return;
        }
        self.depth.set(old - 1);
    }
    fn push_str(&self, s: impl AsRef<str>) {
        let s = s.as_ref();
        self.ret.borrow_mut().push_str(s);
    }

    fn no_space_or_new_line_for_comment(&self) -> bool {
        if self.ret.borrow().chars().last().is_some() {
            !self.ret.borrow().ends_with('\n')
                && !self.ret.borrow().ends_with(' ')
                && !self.ret.borrow().ends_with('(')
        } else {
            false
        }
    }

    /// 缩进
    fn indent(&self) {
        self.push_str(
            " ".to_string()
                .repeat(self.depth.get() * self.local_cfg.indent_size)
                .as_str(),
        );
    }

    fn translate_line(&self, pos: u32) -> u32 {
        self.line_mapping.translate(pos, pos).unwrap().start.line
    }

    /// analyzer a `Nested` token tree.
    fn analyze_token_tree_delimiter(
        token_tree: &[TokenTree],
    ) -> (
        Option<Delimiter>, // if this is a `Delimiter::Semicolon` we can know this is a function body or etc.
        bool,              // has a `:`
    ) {
        let mut d = None;
        let mut has_colon = false;
        for t in token_tree.iter() {
            match t {
                TokenTree::SimpleToken {
                    content,
                    pos: _,
                    tok: _,
                    note: _,
                } => match content.as_str() {
                    ";" => {
                        d = Some(Delimiter::Semicolon);
                    }
                    "," => {
                        if d.is_none() {
                            // Somehow `;` has high priority.
                            d = Some(Delimiter::Comma);
                        }
                    }
                    ":" => {
                        has_colon = true;
                    }
                    _ => {}
                },
                TokenTree::Nested { .. } => {}
            }
        }
        (d, has_colon)
    }

    fn analyzer_token_tree_start_pos_(&self, ret: &mut u32, token_tree: &TokenTree) {
        match token_tree {
            TokenTree::SimpleToken {
                content: _, pos, ..
            } => {
                *ret = *pos;
            }
            TokenTree::Nested {
                elements: _, kind, ..
            } => {
                *ret = kind.start_pos;
            }
        }
    }

    fn process_same_line_comment(
        &self,
        add_line_comment_pos: u32,
        process_tail_comment_of_line: bool,
    ) {
        let cur_line = self.cur_line.get();
        let mut call_new_line = false;
        for c in &self.comments[self.comments_index.get()..] {
            if !process_tail_comment_of_line && c.start_offset > add_line_comment_pos {
                break;
            }

            if self.translate_line(add_line_comment_pos) != self.translate_line(c.start_offset) {
                break;
            }
            // tracing::debug!("self.translate_line(c.start_offset) = {}, self.cur_line.get() = {}", self.translate_line(c.start_offset), self.cur_line.get());
            // tracing::debug!("add a new line[{:?}], meet comment", c.content);
            // if (self.translate_line(c.start_offset) - self.cur_line.get()) > 1 {
            //     tracing::debug!("add a black line");
            //     self.new_line(None);
            // }
            // self.push_str(c.content.as_str());
            let kind = c.comment_kind();
            let fmted_cmt_str = c.format_comment(
                kind,
                self.depth.get() * self.local_cfg.indent_size,
                0,
                &self.global_cfg,
            );
            // tracing::debug!("fmted_cmt_str in same_line = \n{}", fmted_cmt_str);
            /*
            let buffer = self.ret.clone();
            if !buffer.clone().borrow().chars().last().unwrap_or(' ').is_ascii_whitespace()
            && !buffer.clone().borrow().chars().last().unwrap_or(' ').eq(&'('){
                self.push_str(" ");
                // insert 2 black space before '//'
                // if let Some(_) = fmted_cmt_str.find("//") {
                //     self.push_str(" ");
                // }
            }
            */
            if self.no_space_or_new_line_for_comment() {
                self.push_str(" ");
            }

            self.push_str(fmted_cmt_str);
            match kind {
                CommentKind::BlockComment => {
                    let end = c.start_offset + (c.content.len() as u32);
                    let line_start = self.translate_line(c.start_offset);
                    let line_end = self.translate_line(end);
                    if line_start != line_end {
                        tracing::debug!("in new_line, add CommentKind::BlockComment");
                        self.new_line(None);
                        call_new_line = true;
                    }
                }
                _ => {
                    // tracing::debug!("-- process_same_line_comment, add CommentKind::_({})", c.content);
                    self.new_line(None);
                    call_new_line = true;
                }
            }
            self.comments_index.set(self.comments_index.get() + 1);
            self.cur_line
                .set(self.translate_line(c.start_offset + (c.content.len() as u32) - 1));
        }
        if cur_line != self.cur_line.get() || call_new_line {
            tracing::debug!("success new line, return <<<<<<<<<<<<<<<<< \n");
            return;
        }
        self.push_str("\n");
        self.indent();
    }

    fn new_line(&self, add_line_comment_option: Option<u32>) {
        let (add_line_comment, b_add_comment) = match add_line_comment_option {
            Some(add_line_comment) => (add_line_comment, true),
            _ => (0, false),
        };
        if !b_add_comment {
            self.push_str("\n");
            self.indent();
            return;
        }
        self.process_same_line_comment(add_line_comment, false);
    }
}

impl Format {
    fn last_line(&self) -> String {
        self.ret
            .borrow()
            .lines()
            .last()
            .map(|x| x.to_string())
            .unwrap_or_default()
    }

    fn tok_suitable_for_new_line(tok: Tok, note: Option<Note>, next: Option<&TokenTree>) -> bool {
        // special case
        if next
            .and_then(|x| match x {
                TokenTree::SimpleToken {
                    content: _,
                    pos: _,
                    tok: _,
                    note: _,
                } => None,
                TokenTree::Nested {
                    elements: _,
                    kind,
                    note: _,
                } => Some(kind.kind == NestKind_::Type),
            })
            .unwrap_or_default()
        {
            // tracing::debug!("tok_suitable_for_new_line ret false");
            return false;
        }
        let is_bin = note.map(|x| x == Note::BinaryOP).unwrap_or_default();
        let ret = match tok {
            Tok::Less | Tok::Amp | Tok::Star | Tok::Greater if is_bin => true,
            Tok::ExclaimEqual
            | Tok::Percent
            | Tok::AmpAmp
            | Tok::Plus
            | Tok::Minus
            | Tok::Period
            | Tok::Slash
            | Tok::LessEqual
            | Tok::LessLess
            | Tok::Equal
            | Tok::EqualEqual
            | Tok::EqualEqualGreater
            | Tok::LessEqualEqualGreater
            | Tok::GreaterEqual
            | Tok::GreaterGreater
            | Tok::NumValue => true,
            _ => false,
        };
        tracing::debug!("tok_suitable_for_new_line ret = {}", ret);
        ret
    }

    fn get_cur_line_len(&self) -> usize {
        let last_ret = self.last_line();
        let mut tokens_len = 0;
        let mut special_key = false;
        let mut lexer = Lexer::new(&last_ret, FileHash::empty());
        lexer.advance().unwrap();
        while lexer.peek() != Tok::EOF {
            tokens_len += lexer.content().len();
            if !special_key {
                special_key = matches!(
                    lexer.peek(),
                    Tok::If
                        | Tok::LBrace
                        | Tok::Module
                        | Tok::Script
                        | Tok::Struct
                        | Tok::Fun
                        | Tok::Public
                        | Tok::Inline
                        | Tok::Colon
                        | Tok::Spec
                );
            }
            lexer.advance().unwrap();
        }

        if special_key {
            tokens_len
        } else {
            last_ret.len()
        }
    }

    fn judge_change_new_line_when_over_limits(
        &self,
        tok: Tok,
        note: Option<Note>,
        next: Option<&TokenTree>,
    ) -> bool {
        self.get_cur_line_len() + tok.to_string().len() > self.global_cfg.max_width()
            && Self::tok_suitable_for_new_line(tok, note, next)
    }

    fn remove_trailing_whitespaces(&mut self) {
        let ret_copy = self.ret.clone().into_inner();
        let lines = ret_copy.lines();
        let result: String = lines
            .collect::<Vec<_>>()
            .iter()
            .map(|line| line.trim_end_matches(|c| c == ' '))
            .collect::<Vec<_>>()
            .join("\n");
        *self.ret.borrow_mut() = result;
    }

    fn process_last_empty_line(&mut self) {
        let ret_copy = self.ret.clone().into_inner();
        let mut lines = ret_copy.lines().collect::<Vec<&str>>();
        let last_line = lines.last().unwrap_or(&"");

        if last_line.is_empty() {
            while lines.len() > 1 && lines[lines.len() - 2].is_empty() {
                lines.pop();
            }
        } else {
            lines.push("");
        }

        *self.ret.borrow_mut() = lines.join("\n");
    }
}

pub fn format_entry(content: impl AsRef<str>, config: Config) -> Result<String, Diagnostics> {
    let mut timer = Timer::start();
    let content = content.as_ref();

    let mut full_fmt = Format::new(
        config.clone(),
        content,
        FormatContext::new(content.to_string(), FormatEnv::FormatDefault),
    );

    full_fmt.generate_token_tree(content)?;
    timer = timer.done_parsing();

    let result = full_fmt.format_token_trees();
    timer = timer.done_formatting();
    if config.verbose() == Verbosity::Verbose {
        println!(
            "Spent {0:.3} secs in the parsing phase, and {1:.3} secs in the formatting phase",
            timer.get_parse_time(),
            timer.get_format_time(),
        );
    }
    Ok(result)
}

// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use core::panic;
use std::cmp::Ordering;
use std::collections::HashSet;

use move_compiler::parser::ast::Definition;
use move_compiler::parser::ast::*;
use move_compiler::parser::lexer::{Lexer, Tok};
use move_compiler::shared::Identifier;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
pub enum NestKind_ {
    /// ()
    ParentTheses,
    /// []  
    Bracket,
    /// {}
    Brace,
    /// type parameter like  A<B>
    Type,
    /// lambda like |a , b|
    Lambda,
}

#[derive(Clone, Copy, serde::Serialize, Debug)]
pub struct NestKind {
    pub kind: NestKind_,
    pub start_pos: u32,
    pub end_pos: u32,
}

impl NestKind {
    pub fn start_token_tree(&self) -> TokenTree {
        TokenTree::SimpleToken {
            content: self.kind.start_tok().to_string(),
            pos: self.start_pos,
            tok: self.kind.start_tok(),
            note: None,
        }
    }

    pub fn end_token_tree(&self) -> TokenTree {
        TokenTree::SimpleToken {
            content: self.kind.end_tok().to_string(),
            pos: self.end_pos,
            tok: self.kind.end_tok(),
            note: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Delimiter {
    Semicolon,
    Comma,
}

impl Delimiter {
    pub fn to_static_str(self) -> &'static str {
        match self {
            Delimiter::Semicolon => ";",
            Delimiter::Comma => ",",
        }
    }
}
impl NestKind_ {
    pub fn is_nest_start(tok: Tok) -> Option<NestKind_> {
        match tok {
            Tok::LParen => Some(Self::ParentTheses),
            Tok::LBracket => Some(Self::Bracket),
            Tok::LBrace => Some(Self::Brace),
            Tok::Less => Some(Self::Type),
            Tok::Pipe => Some(Self::Lambda),
            _ => None,
        }
    }

    pub const fn start_tok(self) -> Tok {
        match self {
            NestKind_::ParentTheses => Tok::LParen,
            NestKind_::Bracket => Tok::LBracket,
            NestKind_::Brace => Tok::LBrace,
            NestKind_::Type => Tok::Less,
            NestKind_::Lambda => Tok::Pipe,
        }
    }

    pub fn end_tok(self) -> Tok {
        match self {
            NestKind_::ParentTheses => Tok::RParen,
            NestKind_::Bracket => Tok::RBracket,
            NestKind_::Brace => Tok::RBrace,
            NestKind_::Type => Tok::Greater,
            NestKind_::Lambda => Tok::Pipe,
        }
    }
}

#[derive(Clone, serde::Serialize)]
pub enum TokenTree {
    SimpleToken {
        content: String,
        #[serde(skip_serializing)]
        pos: u32, // start offset in file buffer.
        #[serde(skip_serializing)]
        tok: Tok,
        #[serde(skip_serializing)]
        note: Option<Note>,
    },
    Nested {
        elements: Vec<TokenTree>,
        kind: NestKind,
        #[serde(skip_serializing)]
        note: Option<Note>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Note {
    /// binary op like `+` , `*`
    BinaryOP,
    /// This is a struct definition.
    StructDefinition,
    /// This is a function body.
    FunBody,
    /// This is a apply name in `apply` statement.
    ApplyName,
    /// This is a address that contained modules.
    ModuleAddress,
}

impl Default for TokenTree {
    fn default() -> Self {
        TokenTree::SimpleToken {
            content: "".to_string(),
            pos: 0,
            tok: Tok::EOF,
            note: None,
        }
    }
}

impl TokenTree {
    pub fn get_note(&self) -> Option<Note> {
        match self {
            TokenTree::SimpleToken {
                content: _,
                pos: _,
                tok: _,
                note,
            } => *note,
            TokenTree::Nested {
                elements: _,
                kind: _,
                note,
            } => *note,
        }
    }
    pub fn end_pos(&self) -> u32 {
        match self {
            TokenTree::SimpleToken {
                content,
                pos,
                tok: _,
                note: _,
            } => pos + (content.len() as u32),
            TokenTree::Nested {
                elements: _,
                kind,
                note: _,
            } => kind.end_pos,
        }
    }

    pub fn is_pound(&self) -> bool {
        match self {
            TokenTree::SimpleToken { tok, .. } => *tok == Tok::NumSign,
            TokenTree::Nested { .. } => false,
        }
    }

    pub fn simple_str(&self) -> Option<&str> {
        match self {
            TokenTree::SimpleToken {
                content,
                pos: _,
                tok: _,
                note: _,
            } => Some(content.as_str()),
            TokenTree::Nested {
                elements: _,
                kind: _,
                note: _,
            } => None,
        }
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    defs: &'a Vec<Definition>,
    type_lambda_pair: Vec<(u32, u32)>,
    type_lambda_pair_index: usize,
    struct_definitions: Vec<(u32, u32)>,
    bin_op: HashSet<u32>,
    fun_body: HashSet<u32>, // start pos.
    apple_name: HashSet<u32>,
    address_module: Vec<(u32, u32)>,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>, defs: &'a Vec<Definition>) -> Self {
        let mut x = Self {
            lexer,
            defs,
            type_lambda_pair: Default::default(),
            type_lambda_pair_index: 0,
            struct_definitions: vec![],
            bin_op: Default::default(),
            fun_body: Default::default(),
            apple_name: Default::default(),
            address_module: vec![],
        };
        x.collect_various_information();
        x
    }
}

impl<'a> Parser<'a> {
    fn add_simple_note(&self, pos: u32) -> Option<Note> {
        if self.bin_op.contains(&pos) {
            return Some(Note::BinaryOP);
        }
        if self.apple_name.contains(&pos) {
            return Some(Note::ApplyName);
        }
        None
    }

    pub fn parse_tokens(mut self) -> Vec<TokenTree> {
        let mut ret = vec![];
        self.lexer.advance().unwrap();
        while self.lexer.peek() != Tok::EOF {
            if let Some(kind) = self.is_nest_start() {
                ret.push(self.parse_nested(kind));
                continue;
            }
            ret.push(TokenTree::SimpleToken {
                content: self.lexer.content().to_string(),
                pos: self.lexer.start_loc() as u32,
                tok: self.lexer.peek(),
                note: self.add_simple_note(self.lexer.start_loc() as u32),
            });

            self.lexer.advance().unwrap();
        }
        ret
    }

    fn is_nest_start(&mut self) -> Option<NestKind_> {
        let t = match NestKind_::is_nest_start(self.lexer.peek()) {
            Some(x) => x,
            None => return None,
        };
        match t {
            NestKind_::Type | NestKind_::Lambda => {
                let pos = self.lexer.start_loc() as u32;
                // try drop
                for (_, end) in &self.type_lambda_pair[self.type_lambda_pair_index..] {
                    if end < &pos {
                        self.type_lambda_pair_index += 1;
                    } else {
                        break;
                    }
                }

                if let Some((start, end)) = self.type_lambda_pair[self.type_lambda_pair_index..]
                    .iter()
                    .next()
                {
                    if &pos >= start && &pos <= end {
                        return Some(t);
                    } else {
                        return None;
                    }
                }
                None
            }
            _ => Some(t),
        }
    }

    fn parse_nested(&mut self, kind: NestKind_) -> TokenTree {
        debug_assert!(self.lexer.peek() == kind.start_tok());
        let start = self.lexer.start_loc() as u32;
        self.lexer.advance().unwrap();
        let mut ret = vec![];
        let mut note = None;
        if self
            .struct_definitions
            .iter()
            .any(|x| x.0 <= start && x.1 >= start)
            && kind == NestKind_::Brace
        {
            note = Some(Note::StructDefinition);
        }
        if self.fun_body.contains(&start) {
            note = Some(Note::FunBody);
        }
        for (addr, modname) in self.address_module.clone() {
            if addr < start && start < modname {
                note = Some(Note::ModuleAddress);
                break;
            }
        }

        while self.lexer.peek() != Tok::EOF {
            if self.lexer.peek() == kind.end_tok() {
                break;
            }
            if let Some(kind) = self.is_nest_start() {
                ret.push(self.parse_nested(kind));
                continue;
            }
            if kind == NestKind_::Type && self.lexer.peek() == Tok::GreaterGreater {
                self.adjust_token(Tok::Greater);
                break;
            }
            ret.push(TokenTree::SimpleToken {
                content: self.lexer.content().to_string(),
                pos: self.lexer.start_loc() as u32,
                tok: self.lexer.peek(),
                note: self.add_simple_note(self.lexer.start_loc() as u32),
            });
            self.lexer.advance().unwrap();
        }
        debug_assert_eq!(self.lexer.peek(), kind.end_tok());
        let end = self.lexer.start_loc() as u32;
        self.lexer.advance().unwrap();
        TokenTree::Nested {
            elements: ret,
            kind: NestKind {
                kind,
                start_pos: start,
                end_pos: end,
            },
            note,
        }
    }
}

impl<'a> Parser<'a> {
    // While parsing a list and expecting a ">" token to mark the end, replace
    // a ">>" token with the expected ">". This handles the situation where there
    // are nested type parameters that result in two adjacent ">" tokens, e.g.,
    // "A<B<C>>".
    fn adjust_token(&mut self, end_token: Tok) {
        if self.lexer.peek() == Tok::GreaterGreater && end_token == Tok::Greater {
            self.lexer.replace_token(Tok::Greater, 1);
        }
    }
}

impl<'a> Parser<'a> {
    fn collect_various_information(&mut self) {
        for d in self.defs.iter() {
            collect_definition(self, d);
        }
        self.type_lambda_pair
            .iter()
            .for_each(|x| debug_assert!(x.0 <= x.1));

        self.type_lambda_pair.sort_by(|x, y| {
            debug_assert!(x.0.cmp(&y.0) != Ordering::Equal, "{:?}?{:?}", x, y);
            if x.0.cmp(&y.0) == Ordering::Greater {
                Ordering::Greater
            } else {
                x.1.cmp(&y.1)
            }
        });

        fn collect_definition(p: &mut Parser, d: &Definition) {
            match d {
                Definition::Module(x) => collect_module(p, x),
                Definition::Address(addr) => {
                    for x in addr.modules.iter() {
                        p.address_module.push((addr.loc.start(), x.loc.start()));
                        collect_module(p, x);
                    }
                }
                Definition::Script(x) => collect_script(p, x),
            }
        }

        fn collect_script(p: &mut Parser, d: &Script) {
            collect_function(p, &d.function);
            for s in d.specs.iter() {
                collect_spec(p, s);
            }
        }

        fn collect_module(p: &mut Parser, d: &ModuleDefinition) {
            for m in d.members.iter() {
                match &m {
                    ModuleMember::Function(x) => collect_function(p, x),
                    ModuleMember::Struct(x) => collect_struct(p, x),
                    ModuleMember::Use(_) => {}
                    ModuleMember::Friend(_) => {}
                    ModuleMember::Constant(x) => {
                        collect_const(p, x);
                    }
                    ModuleMember::Spec(s) => collect_spec(p, s),
                }
            }
        }

        fn collect_struct(p: &mut Parser, s: &StructDefinition) {
            p.type_lambda_pair.push((s.loc.start(), s.loc.end()));
            p.struct_definitions.push((s.loc.start(), s.loc.end()));
        }

        fn collect_seq_item(p: &mut Parser, s: &SequenceItem) {
            match &s.value {
                SequenceItem_::Seq(e) => collect_expr(p, e),
                SequenceItem_::Declare(_, ty) => {
                    if let Some(ty) = ty {
                        collect_ty(p, ty);
                    }
                }
                SequenceItem_::Bind(_, ty, e) => {
                    if let Some(ty) = ty {
                        collect_ty(p, ty);
                    }
                    collect_expr(p, e);
                }
            }
        }
        fn collect_seq(p: &mut Parser, s: &Sequence) {
            for s in s.1.iter() {
                collect_seq_item(p, s);
            }
            if let Some(t) = s.3.as_ref() {
                collect_expr(p, t);
            }
        }

        fn collect_expr(p: &mut Parser, e: &Exp) {
            match &e.value {
                Exp_::Value(_) => {}
                Exp_::Move(_) => {}
                Exp_::Copy(_) => {}
                Exp_::Name(name, tys) => {
                    if tys.is_some() {
                        p.type_lambda_pair.push((name.loc.end(), e.loc.end()));
                    }
                }
                Exp_::Call(name, _, _tys, es) => {
                    p.type_lambda_pair.push((name.loc.end(), es.loc.start()));
                    es.value.iter().for_each(|e| collect_expr(p, e));
                }
                Exp_::Pack(name, _tys, es) => {
                    if let Some(e) = es.get(0) {
                        p.type_lambda_pair.push((name.loc.end(), e.0.loc().start()));
                    }
                    es.iter().for_each(|e| collect_expr(p, &e.1));
                }
                Exp_::Vector(name_loc, _tys, es) => {
                    p.type_lambda_pair.push((name_loc.end(), es.loc.start()));
                    es.value.iter().for_each(|e| collect_expr(p, e));
                }
                Exp_::IfElse(c, then_, eles_) => {
                    collect_expr(p, c.as_ref());
                    collect_expr(p, then_.as_ref());
                    if let Some(else_) = eles_ {
                        collect_expr(p, else_.as_ref());
                    }
                }
                Exp_::While(e, then_) => {
                    collect_expr(p, e.as_ref());
                    collect_expr(p, then_.as_ref());
                }
                Exp_::Loop(b) => {
                    collect_expr(p, b.as_ref());
                }
                Exp_::Block(b) => collect_seq(p, b),

                Exp_::Lambda(b, e) => {
                    p.type_lambda_pair.push((b.loc.start(), b.loc.end()));
                    collect_expr(p, e.as_ref());
                }
                Exp_::Quant(_, _, es, e1, e2) => {
                    es.iter().for_each(|e| {
                        for e in e.iter() {
                            collect_expr(p, e)
                        }
                    });
                    if let Some(t) = e1 {
                        collect_expr(p, t.as_ref());
                    }
                    collect_expr(p, e2.as_ref());
                }
                Exp_::ExpList(es) => {
                    es.iter().for_each(|e| collect_expr(p, e));
                }
                Exp_::Unit => {}
                Exp_::Assign(l, r) => {
                    collect_expr(p, l.as_ref());
                    collect_expr(p, r.as_ref());
                }
                Exp_::Return(e) => {
                    if let Some(t) = e {
                        collect_expr(p, t.as_ref());
                    }
                }
                Exp_::Abort(e) => {
                    collect_expr(p, e.as_ref());
                }
                Exp_::Break => {}
                Exp_::Continue => {}
                Exp_::Dereference(e) => {
                    collect_expr(p, e.as_ref());
                }
                Exp_::UnaryExp(_, e) => {
                    collect_expr(p, e.as_ref());
                }
                Exp_::BinopExp(l, op, r) => {
                    p.bin_op.insert(op.loc.start());
                    collect_expr(p, l.as_ref());
                    collect_expr(p, r.as_ref());
                }
                Exp_::Borrow(_, e) => {
                    collect_expr(p, e.as_ref());
                }
                Exp_::Dot(e, _) => {
                    collect_expr(p, e.as_ref());
                }
                Exp_::Index(e, i) => {
                    collect_expr(p, e.as_ref());
                    collect_expr(p, i.as_ref());
                }
                Exp_::Cast(e, ty) => {
                    collect_expr(p, e.as_ref());
                    collect_ty(p, ty);
                }
                Exp_::Annotate(e, ty) => {
                    collect_expr(p, e.as_ref());
                    collect_ty(p, ty);
                }
                Exp_::Spec(s) => collect_spec(p, s),
                Exp_::UnresolvedError => {
                    unreachable!()
                }
            }
        }

        fn collect_spec(p: &mut Parser, spec_block: &SpecBlock) {
            match &spec_block.value.target.value {
                SpecBlockTarget_::Code => {}
                SpecBlockTarget_::Module => {}
                SpecBlockTarget_::Member(_, _) | SpecBlockTarget_::Schema(_, _) => {
                    p.type_lambda_pair.push((
                        spec_block.value.target.loc.start(),
                        spec_block.value.target.loc.end(),
                    ));
                }
            }

            for m in spec_block.value.members.iter() {
                match &m.value {
                    SpecBlockMember_::Condition {
                        kind,
                        properties: _,
                        exp,
                        additional_exps,
                    } => {
                        p.type_lambda_pair.push((kind.loc.start(), kind.loc.end()));
                        collect_expr(p, exp);
                        additional_exps.iter().for_each(|e| collect_expr(p, e));
                    }
                    SpecBlockMember_::Function {
                        uninterpreted: _,
                        name,
                        signature,
                        body,
                    } => {
                        p.type_lambda_pair
                            .push((name.0.loc.end(), signature.return_type.loc.end()));
                        match &body.value {
                            FunctionBody_::Defined(s) => collect_seq(p, s),
                            FunctionBody_::Native => {}
                        }
                    }
                    SpecBlockMember_::Variable {
                        is_global: _,
                        name,
                        type_parameters: _,
                        type_: _,
                        init,
                    } => {
                        if let Some(init) = init {
                            p.type_lambda_pair
                                .push((name.loc.start(), init.loc.start() - 1))
                        } else {
                            p.type_lambda_pair.push((name.loc.start(), m.loc.end()));
                        }
                    }

                    SpecBlockMember_::Let {
                        name: _,
                        post_state: _,
                        def,
                    } => collect_expr(p, def),
                    SpecBlockMember_::Update { lhs, rhs } => {
                        collect_expr(p, lhs);
                        collect_expr(p, rhs);
                    }
                    SpecBlockMember_::Include { properties: _, exp } => {
                        collect_expr(p, exp);
                    }
                    SpecBlockMember_::Apply {
                        exp,
                        patterns,
                        exclusion_patterns,
                    } => {
                        for pat in patterns.iter().chain(exclusion_patterns.iter()) {
                            for n in &pat.value.name_pattern {
                                p.apple_name.insert(n.loc.start());
                            }
                            if let Some(x) = pat.value.name_pattern.last() {
                                p.type_lambda_pair.push((x.loc.end(), pat.loc.end()));
                            }
                        }
                        collect_expr(p, exp);
                    }
                    SpecBlockMember_::Pragma { properties: _ } => {}
                }
            }
        }
        fn collect_const(p: &mut Parser, c: &Constant) {
            collect_ty(p, &c.signature);
            collect_expr(p, &c.value);
        }

        fn collect_function(p: &mut Parser, d: &Function) {
            p.type_lambda_pair.push((
                d.name.0.loc.start(),
                match &d.body.value {
                    FunctionBody_::Defined(_x) => d.body.loc.start(),
                    FunctionBody_::Native => d.loc.end(),
                },
            ));

            match &d.body.value {
                FunctionBody_::Defined(s) => {
                    p.fun_body.insert(d.body.loc.start());
                    collect_seq(p, s);
                }
                FunctionBody_::Native => {}
            }
        }

        fn collect_ty(p: &mut Parser, ty: &Type) {
            p.type_lambda_pair.push((ty.loc.start(), ty.loc.end()))
        }
    }
}

fn analyzer_token_tree_length_(ret: &mut usize, token_tree: &TokenTree, max: usize) {
    match token_tree {
        TokenTree::SimpleToken { content, .. } => {
            *ret += content.len();
        }
        TokenTree::Nested { elements, .. } => {
            for t in elements.iter() {
                analyzer_token_tree_length_(ret, t, max);
                if *ret > max {
                    return;
                }
            }
            *ret += 2; // for delimiter.
        }
    }
}

/// analyzer How long is list of token_tree
pub(crate) fn analyze_token_tree_length(token_tree: &[TokenTree], max: usize) -> usize {
    let mut ret = usize::default();
    for t in token_tree.iter() {
        analyzer_token_tree_length_(&mut ret, t, max);
        if ret > max {
            return ret;
        }
    }
    ret
}

// ===================================================================================================
#[derive(Default, Debug)]
pub struct CommentExtrator {
    pub comments: Vec<Comment>,
}

#[derive(Debug)]
pub struct Comment {
    pub start_offset: u32,
    pub content: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    /// "//"
    InlineComment,
    /// "///"
    DocComment,
    /// "/**/"
    BlockComment,
}

/// A comment extractor ,extrat all comment from move file
/// include the start  and end tokens `//`,`*/`,etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentExtratorErr {
    NewLineInQuote,
    NotTermialState(ExtratorCommentState),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExtratorCommentState {
    /// init state
    Init,
    /// `/` has been seen,maybe a comment.
    OneSlash,
    /// `///` has been seen,inline comment.
    InlineComment,
    /// `/*` has been seen,block comment.
    BlockComment,
    /// in state `BlockComment`,`*` has been seen,maybe exit the `BlockComment`.
    OneStar,
    /// `"` has been seen.
    Quote,
}

impl CommentExtrator {
    pub fn new(content: &str) -> Result<Self, CommentExtratorErr> {
        if content.len() <= 1 {
            return Ok(Self::default());
        }
        let content = content.as_bytes();

        let mut state = ExtratorCommentState::Init;
        const NEW_LINE: u8 = 10;
        const SLASH: u8 = 47;
        const STAR: u8 = 42;
        const BLACK_SLASH: u8 = 92;
        const QUOTE: u8 = 34;
        let mut depth = 0;
        let mut comments = Vec::new();
        let mut comment = Vec::new();
        let last_index = content.len() - 1;
        let mut index = 0;

        macro_rules! make_comment {
            () => {
                comments.push(Comment {
                    start_offset: (index as u32) + 1 - (comment.len() as u32),
                    content: String::from_utf8(comment.clone()).unwrap(),
                });
                comment.clear();

                if state == ExtratorCommentState::InlineComment {
                    if depth == 0 {
                        state = ExtratorCommentState::Init;
                    } else {
                        state = ExtratorCommentState::BlockComment;
                    }
                } else {
                    if depth <= 1 {
                        depth = 0;
                        state = ExtratorCommentState::Init;
                    } else {
                        depth -= 1;
                        state = ExtratorCommentState::BlockComment;
                    }
                }
            };
        }
        // tracing::info!("xxxx:{}", content.len());
        while index <= last_index {
            let c = content.get(index).unwrap();
            // tracing::info!(
            //     "index:{} state:{:?} c:{:?} last:{}",
            //     index, state, *c as char, last_index
            // );
            match state {
                ExtratorCommentState::Init => match *c {
                    SLASH => {
                        state = ExtratorCommentState::OneSlash;
                    }
                    QUOTE => {
                        state = ExtratorCommentState::Quote;
                    }
                    _ => {}
                },
                ExtratorCommentState::OneSlash => {
                    if *c == SLASH {
                        comment.push(SLASH);
                        comment.push(SLASH);
                        if depth == 0 {
                            state = ExtratorCommentState::InlineComment;
                        } else {
                            state = ExtratorCommentState::BlockComment;
                        }
                    } else if *c == STAR {
                        comment.push(SLASH);
                        comment.push(STAR);
                        depth += 1;
                        state = ExtratorCommentState::BlockComment;
                    } else if depth == 0 {
                        state = ExtratorCommentState::Init;
                    } else {
                        state = ExtratorCommentState::BlockComment;
                    }
                }
                ExtratorCommentState::BlockComment => {
                    if *c == STAR {
                        state = ExtratorCommentState::OneStar;
                    } else if *c == SLASH {
                        state = ExtratorCommentState::OneSlash;
                    } else {
                        comment.push(*c);
                    }
                }
                ExtratorCommentState::OneStar => {
                    if *c == SLASH {
                        comment.push(STAR);
                        comment.push(SLASH);
                        make_comment!();
                    } else if *c == STAR {
                        comment.push(STAR);
                    } else {
                        comment.push(STAR);
                        comment.push(*c);
                        state = ExtratorCommentState::BlockComment;
                    }
                }
                ExtratorCommentState::InlineComment => {
                    if *c == NEW_LINE || index == last_index {
                        if *c != NEW_LINE {
                            comment.push(*c);
                        }
                        make_comment!();
                    } else {
                        comment.push(*c);
                    }
                }
                ExtratorCommentState::Quote => {
                    // handle \" or handle \\
                    if *c == BLACK_SLASH
                        && content
                            .get(index + 1)
                            .map(|x| *x == QUOTE || *x == BLACK_SLASH)
                            .unwrap_or(false)
                    {
                        index += 2;
                        continue;
                    } else if *c == QUOTE {
                        state = ExtratorCommentState::Init;
                    } else if *c == NEW_LINE {
                        // return Err(CommentExtratorErr::NewLineInQuote);
                        panic!("1")
                    }
                }
            };
            index += 1;
        }
        if state != ExtratorCommentState::Init {
            return Err(CommentExtratorErr::NotTermialState(state));
        }
        Ok(Self { comments })
    }
}

#[cfg(test)]
mod comment_test {
    use crate::tools::syntax::parse_file_string;
    use move_command_line_common::files::FileHash;
    use move_compiler::{shared::CompilationEnv, Flags};
    use std::collections::BTreeSet;

    use super::*;
    #[test]
    fn token_tree_to_json() {
        let content = r#"module 0x1::xxx{
            public entry fun object_vec(msg: String, objs: vector<Object<ModuleData>>) acquires ModuleData {
                vector::for_each(objs,|o| {  });
            }
          }"#;
        let filehash = FileHash::empty();
        let attrs: BTreeSet<String> = BTreeSet::new();
        let mut env = CompilationEnv::new(Flags::testing(), attrs);
        let (defs, _) = parse_file_string(&mut env, filehash, content).unwrap();
        let lexer = Lexer::new(&content, filehash);
        let parse = Parser::new(lexer, &defs);
        let token_tree = parse.parse_tokens();
        let s = serde_json::to_string(&token_tree).unwrap();
        // check this using some online json tool.
        tracing::info!("json:{}", s);
    }

    #[test]
    fn test_comment_extrator_ok() {
        let x = CommentExtrator::new(
            r#"
        // 111
    // 222
    fdfdf
    // bb
    /* ***** '" 1121 **/
    " \" // iii "
    "// fff"
    /// ggg
    b"\\"
        "#,
        )
        .unwrap();
        let v = vec![
            "// 111",
            "// 222",
            "// bb",
            "/* ***** '\" 1121 **/",
            "/// ggg",
        ];
        for (c1, c2) in v.iter().zip(x.comments.iter()) {
            assert_eq!(*c1, c2.content.as_str());
        }
        assert_eq!(v.len(), x.comments.len());
    }
    #[test]
    fn test_comment_extrator_ok2() {
        let _x = CommentExtrator::new(r#"/* /* 1 */ */"#).unwrap();
    }

    #[test]
    fn test_comment_extrator_err() {
        let v = vec![
            (
                r#" \" "#,
                CommentExtratorErr::NotTermialState(ExtratorCommentState::Quote),
            ),
            (
                r#" \" 
                "#,
                CommentExtratorErr::NewLineInQuote,
            ),
            (
                r#" /*  "#,
                CommentExtratorErr::NotTermialState(ExtratorCommentState::BlockComment),
            ),
        ];

        for (c, err) in v.iter() {
            match CommentExtrator::new(*c) {
                Ok(_) => unreachable!(),
                Err(x) => assert_eq!(x, *err),
            }
        }
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Abstract syntax and parser of the assembler language.
//!
//! Move assembler files (ending in `.masm`) have the following
//! EBNF syntax. Notice that this grammar use indentation
//! expressed as `IND` and newline, denoted as `LF` in the syntax.
//! Moreover, we use `ID` for a simple identifier, and `QID` for
//! a qualified identifier, an optional address constant followed
//! by a sequence of qualified identifiers separated by `::` (as in
//! `0x66::bar::foo`, or `bar::foo`).
//!
//! Notice that line comments are supported and preceded by `//`.
//!
//! ```ignore
//! unit :=
//!   { address_alias LF }
//!   ( "module" QID | "script" ) LF
//!   { "uses" QID [ "as" ID ] }
//!   { struct_def | fun_def }
//!
//! struct_def :=
//!   "struct" ID [ type_args ] LF { field }
//! | "enum" ID [ type_args ] LF variant { variant }
//!
//! field ::=
//!   INDENT ID ":" type LF
//!
//! variant ::=
//!   INDENT ID LF { INDENT ID ":" type LF }
//!
//!
//! fun_def :=
//!   fun_modifier "fun" ID [ type_args ] "(" [ LIST(local) ] ")" [ tuple_type ] LF
//!   { INDENT "local" local LF } { INDENT spec_clause LF } { instruction LF }
//!
//! spec_clause :=
//!   "requires" spec_exp
//! | "aborts_if" spec_exp
//! | "ensures" spec_exp
//! | "modifies" "global" "<" ID ">" "(" spec_exp ")"
//! | "invariant" ID ":" spec_exp        # ID names the loop header label
//!
//! spec_exp :=      # usual precedences; "==>" binds weakest, right-assoc
//!   ( "forall" | "exists" ) ID ":" ID "." spec_exp
//! | spec_exp (" ==> " | "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">="
//!             | "+" | "-" | "*" | "/" | "%") spec_exp
//! | "!" spec_exp
//! | spec_exp "." ID                    # field selection
//! | NUMBER | "true" | "false" | ID | "result" | "result_N"
//! | "old" "(" spec_exp ")"
//! | "global" "<" ID ">" "(" spec_exp ")"
//! | "exists" "<" ID ">" "(" spec_exp ")"
//! | "(" spec_exp ")"
//!
//! fun_modifier :=
//!   [ "#[" attribute "]"
//!   [ "entry" ]
//!   [ "public" | "friend" ]
//!   [ "native" ]
//!
//! attribute := ID
//!
//! local := ID ":" type
//!
//! type :=
//!   "|" [ LIST(type) ] "|" [ tuple_type ]  | simple_type
//!
//! tuple_type :=
//!     type | "(" LIST(type) ")"
//!
//! simple_type :=
//!   QID [ type_args ] | "(" type ")"
//!
//! type_args :=
//!   "<" LIST(type) ">"
//!
//! instruction :=
//!   // Allow lables to prefix instructions without indent
//!   ( INDENT | LOOKAHEAD(ID ":") )
//!   opcode [ LIST(argument) ]
//!
//! opcode := IDENT # current instr set
//!
//! argument :=
//!   VALUE | QID [ type_args ] | type_args
//!```

use crate::value::AsmValue;
use codespan::{RawIndex, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
use move_binary_format::file_format::{FunctionAttribute, Visibility};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    int256::U256,
    language_storage::ModuleId,
};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::{Display, Formatter},
    ops::Range,
    string::ToString,
};
// ==========================================================================================
// Diagnostics

/// Currently we use as locations span's (no filename), but keep this abstracted in case
/// we want to change this.
pub(crate) type Loc = Span;

/// Currently we produce diagnostics for singe files without explicit name.
pub(crate) type Diag = Diagnostic<()>;

/// A result with a set of diagnostics
pub(crate) type AsmResult<A> = Result<A, Vec<Diag>>;

pub(crate) fn error(loc: Loc, message: impl ToString) -> Vec<Diag> {
    vec![Diag::new(Severity::Error)
        .with_labels(vec![Label::primary((), loc)])
        .with_message(message.to_string())]
}

pub(crate) fn loc(range: Range<usize>) -> Loc {
    Loc::new(range.start as RawIndex, range.end as RawIndex)
}

pub(crate) fn map_diag<A>(result: anyhow::Result<A>) -> AsmResult<A> {
    result.map_err(|err| error(loc(0..0), err.to_string()))
}

// ==========================================================================================
// Abstract Syntax

/// Represents the AST for assembler source unit.
#[derive(Debug)]
pub struct Unit {
    /// The name, either script or module.
    pub name: UnitId,
    /// A list of address aliases.
    pub address_aliases: Vec<(Identifier, AccountAddress)>,
    /// A list of module aliases.
    pub module_aliases: Vec<(Identifier, ModuleId)>,
    /// Friend modules
    pub friend_modules: Vec<ModuleId>,
    /// List of struct definitions (including enums, which are technically
    /// a special form of struct).
    pub structs: Vec<Struct>,
    /// List of function definitions.
    pub functions: Vec<Fun>,
}

#[derive(Debug, Clone)]
pub enum UnitId {
    Script,
    Module(ModuleId),
}

impl UnitId {
    pub fn module_opt(&self) -> Option<&ModuleId> {
        match self {
            UnitId::Script => None,
            UnitId::Module(id) => Some(id),
        }
    }
}

#[derive(Debug)]
pub struct Struct {
    pub loc: Loc,
    pub name: Identifier,
    /// Each type parameter is a triple `(name, constraints, is_phantom)`
    pub type_params: Vec<(Identifier, AbilitySet, bool)>,
    pub abilities: AbilitySet,
    pub layout: StructLayout,
}

#[derive(Debug)]
pub enum StructLayout {
    // A struct with a set of field declarations.
    Singleton(Vec<Decl>),
    // An enum with a list of variants, each one
    // represented by a name and a list fields.
    Variants(Vec<(Loc, Identifier, Vec<Decl>)>),
}

#[derive(Debug)]
pub struct Fun {
    pub loc: Loc,
    pub name: Identifier,
    pub visibility: Visibility,
    pub is_entry: bool,
    pub is_native: bool,
    pub attributes: Vec<FunctionAttribute>,
    pub type_params: Vec<(Identifier, AbilitySet)>,
    pub params: Vec<Decl>,
    pub locals: Vec<Decl>,
    pub result: Vec<Type>,
    pub acquires: Vec<Identifier>,
    pub spec_clauses: Vec<SpecClause>,
    pub instrs: Vec<Instruction>,
}

#[derive(Debug)]
pub enum Type {
    Named(PartialIdent, Option<Vec<Type>>),
    Func(Vec<Type>, Vec<Type>, AbilitySet),
    Ref(/*is_mut*/ bool, Box<Type>),
}

#[derive(Debug)]
pub struct PartialIdent {
    /// An optional address.
    pub address: Option<AccountAddress>,
    /// A sequence of name parts, separated via `::`.
    /// The compiler will check how many are valid in a given context.
    pub id_parts: Vec<Identifier>,
}

#[derive(Debug)]
pub struct Decl {
    pub loc: Loc,
    pub name: Identifier,
    pub ty: Type,
}

/// A specification clause attached to a function.  Clauses are surface
/// syntax only: the assembler ignores them; downstream tools (e.g. the
/// Lean formalization's masm frontend) consume them from the AST.
#[derive(Debug)]
pub enum SpecClause {
    Requires(SpecExp),
    AbortsIf(SpecExp),
    Ensures(SpecExp),
    Modifies { resource: Identifier, addr: SpecExp },
    Invariant { label: Identifier, exp: SpecExp },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecQuantKind {
    Forall,
    Exists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
    And,
    Or,
    Implies,
}

/// Specification expressions (see the grammar in the module docs).
#[derive(Debug)]
pub enum SpecExp {
    Number(U256),
    Bool(bool),
    Ident(Identifier),
    Result(usize),
    Old(Box<SpecExp>),
    Global {
        resource: Identifier,
        addr: Box<SpecExp>,
    },
    ResourceExists {
        resource: Identifier,
        addr: Box<SpecExp>,
    },
    Select {
        exp: Box<SpecExp>,
        field: Identifier,
    },
    Not(Box<SpecExp>),
    Binary(SpecBinOp, Box<SpecExp>, Box<SpecExp>),
    Quant {
        kind: SpecQuantKind,
        var: Identifier,
        ty: Identifier,
        body: Box<SpecExp>,
    },
}

#[derive(Debug)]
pub struct Instruction {
    pub loc: Loc,
    pub label: Option<Identifier>,
    pub name: Identifier,
    pub args: Vec<Argument>,
}

#[derive(Debug)]
pub enum Argument {
    Constant(AsmValue),
    Id(PartialIdent, Option<Vec<Type>>),
    Type(Type),
}

// ==========================================================================================
// Specification clause parsing

/// The keywords which can start a specification clause (each has a parse
/// arm in `AsmParser::spec_clause`).
const SPEC_CLAUSE_KEYWORDS: &[&str] =
    &["requires", "aborts_if", "ensures", "modifies", "invariant"];

impl AsmParser {
    fn is_spec_clause_start(&self) -> bool {
        !self.lookahead_special(":") && SPEC_CLAUSE_KEYWORDS.iter().any(|kw| self.is_soft_kw(kw))
    }

    fn spec_clause(&mut self) -> AsmResult<SpecClause> {
        let kw = self.ident()?;
        match kw.as_str() {
            "requires" => Ok(SpecClause::Requires(self.spec_exp()?)),
            "aborts_if" => Ok(SpecClause::AbortsIf(self.spec_exp()?)),
            "ensures" => Ok(SpecClause::Ensures(self.spec_exp()?)),
            "modifies" => {
                self.expect_soft_kw("global")?;
                let (resource, addr) = self.spec_resource_call()?;
                Ok(SpecClause::Modifies { resource, addr })
            },
            "invariant" => {
                let label = self.ident()?;
                self.expect_special(":")?;
                Ok(SpecClause::Invariant {
                    label,
                    exp: self.spec_exp()?,
                })
            },
            _ => Err(error(self.previous_loc, "unexpected spec clause keyword")),
        }
    }

    /// Recognizes the longest specification operator at the current
    /// position.  The scanner produces single-character specials; multi
    /// character operators are recognized by adjacency of their parts
    /// (no whitespace in between).
    fn peek_spec_op(&self) -> Option<(&'static str, usize)> {
        let Token::Special(s0) = &self.next else {
            return None;
        };
        let adjacent = |i: usize, upper: Loc| -> Option<&str> {
            match self.tokens.get(i) {
                Some((l, Token::Special(s))) if l.start() == upper.end() => Some(s.as_str()),
                _ => None,
            }
        };
        let adj1 = adjacent(0, self.next_loc);
        let adj2 = self.tokens.front().and_then(|(l, _)| adjacent(1, *l));
        match (s0.as_str(), adj1, adj2) {
            ("=", Some("="), Some(">")) => Some(("==>", 3)),
            ("=", Some("="), _) => Some(("==", 2)),
            ("!", Some("="), _) => Some(("!=", 2)),
            ("<", Some("="), _) => Some(("<=", 2)),
            (">", Some("="), _) => Some((">=", 2)),
            ("&", Some("&"), _) => Some(("&&", 2)),
            ("|", Some("|"), _) => Some(("||", 2)),
            ("<", _, _) => Some(("<", 1)),
            (">", _, _) => Some((">", 1)),
            ("+", _, _) => Some(("+", 1)),
            ("-", _, _) => Some(("-", 1)),
            ("*", _, _) => Some(("*", 1)),
            ("/", _, _) => Some(("/", 1)),
            ("%", _, _) => Some(("%", 1)),
            ("!", _, _) => Some(("!", 1)),
            (".", _, _) => Some((".", 1)),
            _ => None,
        }
    }

    fn consume_tokens(&mut self, n: usize) -> AsmResult<()> {
        for _ in 0..n {
            self.advance()?;
        }
        Ok(())
    }

    /// `<R>(exp)` after `global`/`exists`/`modifies global`.
    fn spec_resource_call(&mut self) -> AsmResult<(Identifier, SpecExp)> {
        self.expect_special("<")?;
        let resource = self.ident()?;
        self.expect_special(">")?;
        self.expect_special("(")?;
        let addr = self.spec_exp()?;
        self.expect_special(")")?;
        Ok((resource, addr))
    }

    fn spec_exp(&mut self) -> AsmResult<SpecExp> {
        // Quantifiers bind weakest.  `exists<R>(..)` (resource test) is
        // distinguished from the `exists` quantifier by the type argument.
        if (self.is_soft_kw("forall") || self.is_soft_kw("exists"))
            && self.lookahead_spec_quantifier()
        {
            let kind = if self.is_soft_kw("forall") {
                SpecQuantKind::Forall
            } else {
                SpecQuantKind::Exists
            };
            self.advance()?;
            let var = self.ident()?;
            self.expect_special(":")?;
            let ty = self.ident()?;
            self.expect_special(".")?;
            let body = self.spec_exp()?;
            return Ok(SpecExp::Quant {
                kind,
                var,
                ty,
                body: Box::new(body),
            });
        }
        self.spec_implies()
    }

    fn spec_implies(&mut self) -> AsmResult<SpecExp> {
        let lhs = self.spec_or()?;
        if let Some(("==>", n)) = self.peek_spec_op() {
            self.consume_tokens(n)?;
            // Right-associative; the right-hand side may be a quantifier.
            let rhs = self.spec_exp()?;
            Ok(SpecExp::Binary(
                SpecBinOp::Implies,
                Box::new(lhs),
                Box::new(rhs),
            ))
        } else {
            Ok(lhs)
        }
    }

    fn spec_or(&mut self) -> AsmResult<SpecExp> {
        let mut lhs = self.spec_and()?;
        while let Some(("||", n)) = self.peek_spec_op() {
            self.consume_tokens(n)?;
            let rhs = self.spec_and()?;
            lhs = SpecExp::Binary(SpecBinOp::Or, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn spec_and(&mut self) -> AsmResult<SpecExp> {
        let mut lhs = self.spec_cmp()?;
        while let Some(("&&", n)) = self.peek_spec_op() {
            self.consume_tokens(n)?;
            let rhs = self.spec_cmp()?;
            lhs = SpecExp::Binary(SpecBinOp::And, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn spec_cmp(&mut self) -> AsmResult<SpecExp> {
        let lhs = self.spec_add()?;
        let op = match self.peek_spec_op() {
            Some(("==", n)) => Some((SpecBinOp::Eq, n)),
            Some(("!=", n)) => Some((SpecBinOp::Neq, n)),
            Some(("<=", n)) => Some((SpecBinOp::Le, n)),
            Some((">=", n)) => Some((SpecBinOp::Ge, n)),
            Some(("<", n)) => Some((SpecBinOp::Lt, n)),
            Some((">", n)) => Some((SpecBinOp::Gt, n)),
            _ => None,
        };
        if let Some((op, n)) = op {
            self.consume_tokens(n)?;
            let rhs = self.spec_add()?;
            Ok(SpecExp::Binary(op, Box::new(lhs), Box::new(rhs)))
        } else {
            Ok(lhs)
        }
    }

    fn spec_add(&mut self) -> AsmResult<SpecExp> {
        let mut lhs = self.spec_mul()?;
        loop {
            let op = match self.peek_spec_op() {
                Some(("+", n)) => Some((SpecBinOp::Add, n)),
                Some(("-", n)) => Some((SpecBinOp::Sub, n)),
                _ => None,
            };
            let Some((op, n)) = op else {
                return Ok(lhs);
            };
            self.consume_tokens(n)?;
            let rhs = self.spec_mul()?;
            lhs = SpecExp::Binary(op, Box::new(lhs), Box::new(rhs));
        }
    }

    fn spec_mul(&mut self) -> AsmResult<SpecExp> {
        let mut lhs = self.spec_unary()?;
        loop {
            let op = match self.peek_spec_op() {
                Some(("*", n)) => Some((SpecBinOp::Mul, n)),
                Some(("/", n)) => Some((SpecBinOp::Div, n)),
                Some(("%", n)) => Some((SpecBinOp::Mod, n)),
                _ => None,
            };
            let Some((op, n)) = op else {
                return Ok(lhs);
            };
            self.consume_tokens(n)?;
            let rhs = self.spec_unary()?;
            lhs = SpecExp::Binary(op, Box::new(lhs), Box::new(rhs));
        }
    }

    fn spec_unary(&mut self) -> AsmResult<SpecExp> {
        if let Some(("!", n)) = self.peek_spec_op() {
            self.consume_tokens(n)?;
            Ok(SpecExp::Not(Box::new(self.spec_unary()?)))
        } else {
            self.spec_postfix()
        }
    }

    fn spec_postfix(&mut self) -> AsmResult<SpecExp> {
        let mut exp = self.spec_primary()?;
        while let Some((".", n)) = self.peek_spec_op() {
            self.consume_tokens(n)?;
            let field = self.ident()?;
            exp = SpecExp::Select {
                exp: Box::new(exp),
                field,
            };
        }
        Ok(exp)
    }

    fn spec_primary(&mut self) -> AsmResult<SpecExp> {
        if let Token::Number(n) = &self.next {
            let n = *n;
            self.advance()?;
            return Ok(SpecExp::Number(n));
        }
        if self.is_special("(") {
            self.advance()?;
            let exp = self.spec_exp()?;
            self.expect_special(")")?;
            return Ok(exp);
        }
        if !self.is_ident() {
            return Err(error(self.next_loc, "expected a spec expression"));
        }
        if self.is_soft_kw("true") {
            self.advance()?;
            return Ok(SpecExp::Bool(true));
        }
        if self.is_soft_kw("false") {
            self.advance()?;
            return Ok(SpecExp::Bool(false));
        }
        if self.is_soft_kw("old") && self.lookahead_special("(") {
            self.advance()?;
            self.advance()?;
            let exp = self.spec_exp()?;
            self.expect_special(")")?;
            return Ok(SpecExp::Old(Box::new(exp)));
        }
        if self.is_soft_kw("global") && self.lookahead_spec_resource_call() {
            self.advance()?;
            let (resource, addr) = self.spec_resource_call()?;
            return Ok(SpecExp::Global {
                resource,
                addr: Box::new(addr),
            });
        }
        if self.is_soft_kw("exists") && self.lookahead_spec_resource_call() {
            self.advance()?;
            let (resource, addr) = self.spec_resource_call()?;
            return Ok(SpecExp::ResourceExists {
                resource,
                addr: Box::new(addr),
            });
        }
        // Result names are resolved after locals and quantifier binders by
        // the spec consumer.  Treating them specially in the parser would
        // silently shadow legal masm locals named `result`/`result_N`.
        Ok(SpecExp::Ident(self.ident()?))
    }
}

// ==========================================================================================
// Parser

struct AsmParser {
    previous_loc: Loc,
    next_loc: Loc,
    next: Token,
    tokens: VecDeque<(Loc, Token)>,
}

pub fn parse_asm(source: &str) -> AsmResult<Unit> {
    let mut tokens = scan(source.as_bytes())?;
    let (loc, next) = tokens.pop_front().expect("scan returns at least one token");
    AsmParser {
        previous_loc: loc,
        next_loc: loc,
        next,
        tokens,
    }
    .unit()
}

impl AsmParser {
    fn advance(&mut self) -> AsmResult<()> {
        if self.next == Token::End {
            // Ignore -- there are infinite number of `End` from here
            Ok(())
        } else {
            self.previous_loc = self.next_loc;
            (self.next_loc, self.next) = self.tokens.pop_front().unwrap();
            Ok(())
        }
    }

    fn is_tok(&self, tok: &Token) -> bool {
        tok == &self.next
    }

    fn is_soft_kw(&self, kw: &str) -> bool {
        matches!(&self.next, Token::Ident(s) if s == kw)
    }

    fn is_special(&self, sp: &str) -> bool {
        matches!(&self.next, Token::Special(s) if s == sp)
    }

    fn is_indent(&self) -> bool {
        matches!(self.next, Token::Indent(..))
    }

    fn lookahead_special(&self, sp: &str) -> bool {
        !self.tokens.is_empty() && matches!(&self.tokens[0].1, Token::Special(s) if s == sp)
    }

    /// Whether the current identifier is followed by the complete
    /// `<Resource>(` prefix of a specification resource access.  Looking at
    /// only `<` would misparse a legal local named `global` or `exists` in a
    /// comparison such as `global < limit`.
    fn lookahead_spec_resource_call(&self) -> bool {
        matches!(
            (
                self.tokens.front().map(|(_, tok)| tok),
                self.tokens.get(1).map(|(_, tok)| tok),
                self.tokens.get(2).map(|(_, tok)| tok),
                self.tokens.get(3).map(|(_, tok)| tok),
            ),
            (
                Some(Token::Special(open)),
                Some(Token::Ident(_)),
                Some(Token::Special(close)),
                Some(Token::Special(paren)),
            ) if open == "<" && close == ">" && paren == "("
        )
    }

    /// Whether the current `forall`/`exists` is followed by the complete
    /// `name : type .` binder prefix.  This keeps an ordinary local named
    /// `exists` usable in comparisons.
    fn lookahead_spec_quantifier(&self) -> bool {
        matches!(
            (
                self.tokens.front().map(|(_, tok)| tok),
                self.tokens.get(1).map(|(_, tok)| tok),
                self.tokens.get(2).map(|(_, tok)| tok),
                self.tokens.get(3).map(|(_, tok)| tok),
            ),
            (
                Some(Token::Ident(_)),
                Some(Token::Special(colon)),
                Some(Token::Ident(_)),
                Some(Token::Special(dot)),
            ) if colon == ":" && dot == "."
        )
    }

    fn lookahead_special_2(&self, sp: &str) -> bool {
        self.tokens.len() > 1 && matches!(&self.tokens[1].1, Token::Special(s) if s == sp)
    }

    fn lookahead_newline(&self) -> bool {
        !self.tokens.is_empty() && matches!(&self.tokens[0].1, Token::Newline)
    }

    fn lookahead_indent(&self) -> bool {
        !self.tokens.is_empty() && matches!(&self.tokens[0].1, Token::Indent(_))
    }

    #[allow(unused)]
    fn lookahead_soft_kw(&self, kw: &str) -> bool {
        !self.tokens.is_empty() && matches!(&self.tokens[0].1, Token::Ident(s) if s == kw)
    }

    fn expect(&mut self, tok: &Token) -> AsmResult<()> {
        if !self.is_tok(tok) {
            Err(error(
                self.next_loc,
                format!("expected `{}`, found `{}`", tok, self.next),
            ))
        } else {
            self.advance()
        }
    }

    fn expect_special(&mut self, sp: &str) -> AsmResult<()> {
        if !self.is_special(sp) {
            Err(error(
                self.next_loc,
                format!("expected `{}`, found `{}`", sp, self.next),
            ))
        } else {
            self.advance()
        }
    }

    fn expect_soft_kw(&mut self, kw: &str) -> AsmResult<()> {
        if !self.is_soft_kw(kw) {
            Err(error(
                self.next_loc,
                format!("expected `{}`, found `{}`", kw, self.next),
            ))
        } else {
            self.advance()
        }
    }

    fn expect_newline(&mut self) -> AsmResult<()> {
        if self.is_tok(&Token::End) {
            // End of file can serve as newline, but is not consumed
            return Ok(());
        }
        self.expect(&Token::Newline)?;
        // Skip empty lines.
        while self.is_tok(&Token::Newline) {
            self.advance()?
        }
        Ok(())
    }

    fn list<E>(
        &mut self,
        parser: impl Fn(&mut AsmParser) -> AsmResult<E>,
        separator: &str,
    ) -> AsmResult<Vec<E>> {
        let mut result = vec![parser(self)?];
        while self.is_special(separator) {
            self.advance()?;
            result.push(parser(self)?)
        }
        Ok(result)
    }

    fn value(&mut self) -> AsmResult<AsmValue> {
        if self.is_special("-") {
            self.advance()?;
            if let Token::Number(num) = &self.next {
                let num = *num;
                self.advance()?;
                Ok(AsmValue::Number(false, num))
            } else {
                Err(error(self.next_loc, "expected number"))
            }
        } else if let Token::Number(num) = &self.next {
            let num = *num;
            self.advance()?;
            Ok(AsmValue::Number(true, num))
        } else if self.is_special("[") {
            self.advance()?;
            let elems = if self.is_value() {
                self.list(Self::value, ",")?
            } else {
                vec![]
            };
            self.expect_special("]")?;
            Ok(AsmValue::Vector(elems))
        } else {
            Err(error(self.next_loc, "expected value"))
        }
    }

    fn is_value(&self) -> bool {
        matches!(&self.next, Token::Number(..))
            || matches!(&self.next, Token::Special(s) if s == "[" || s == "-")
    }

    fn address(&mut self) -> AsmResult<AccountAddress> {
        if let Token::Number(num) = &self.next {
            let mut bytes = num.to_le_bytes().to_vec();
            bytes.reverse();
            let addr = AccountAddress::from_bytes(bytes).expect("valid address value");
            self.advance()?;
            Ok(addr)
        } else {
            Err(error(self.next_loc, "expected address value"))
        }
    }

    fn ident(&mut self) -> AsmResult<Identifier> {
        if let Token::Ident(str) = &self.next {
            let str = str.clone();
            self.advance()?;
            Ok(Identifier::new_unchecked(str))
        } else {
            Err(error(self.next_loc, "expected identifier"))
        }
    }

    fn is_ident(&self) -> bool {
        matches!(&self.next, Token::Ident(..))
    }

    fn partial_ident(&mut self) -> AsmResult<PartialIdent> {
        let address = if matches!(&self.next, Token::Number(..)) && self.lookahead_special("::") {
            let addr = self.address()?;
            self.advance()?;
            Some(addr)
        } else {
            None
        };
        let id_parts = self.list(Self::ident, "::")?;
        Ok(PartialIdent { address, id_parts })
    }

    fn is_partial_ident(&self) -> bool {
        matches!(&self.next, Token::Number(..)) && self.lookahead_special("::")
            || matches!(&self.next, Token::Ident(..))
    }

    fn type_(&mut self) -> AsmResult<Type> {
        if self.is_special("(") {
            self.advance()?;
            let ty = self.type_()?;
            self.expect_special(")")?;
            Ok(ty)
        } else if self.is_partial_ident() {
            let pid = self.partial_ident()?;
            let ty_args = self.type_args_opt()?;
            Ok(Type::Named(pid, ty_args))
        } else if self.is_special("&") {
            self.advance()?;
            let is_mut = if self.is_soft_kw("mut") {
                self.advance()?;
                true
            } else {
                false
            };
            Ok(Type::Ref(is_mut, Box::new(self.type_()?)))
        } else if self.is_special("|") {
            self.advance()?;
            let arg_tys = if self.is_type() {
                self.type_list()?
            } else {
                vec![]
            };
            self.expect_special("|")?;
            let res_tys = if self.is_type_tuple() {
                self.result_type_tuple()?
            } else {
                vec![]
            };
            let abs = if self.is_soft_kw("has") {
                self.advance()?;
                self.abilities()?
            } else {
                AbilitySet::EMPTY
            };
            Ok(Type::Func(arg_tys, res_tys, abs))
        } else {
            Err(error(self.next_loc, "expected type"))
        }
    }

    fn is_type(&self) -> bool {
        self.is_partial_ident() || self.is_special("&") || self.is_special("(")
    }

    fn type_list(&mut self) -> AsmResult<Vec<Type>> {
        self.list(Self::type_, ",")
    }

    fn result_type_tuple(&mut self) -> AsmResult<Vec<Type>> {
        if self.is_special("(") {
            self.advance()?;
            let res = self.type_list()?;
            self.expect_special(")")?;
            Ok(res)
        } else if self.is_type() || self.is_special("|") {
            Ok(vec![self.type_()?])
        } else {
            Err(error(self.next_loc, "expected type or type tuple"))
        }
    }

    fn is_type_tuple(&self) -> bool {
        self.is_type() || self.is_special("(")
    }

    fn type_args_opt(&mut self) -> AsmResult<Option<Vec<Type>>> {
        if self.is_special("<") {
            self.advance()?;
            let res = self.type_list()?;
            self.expect_special(">")?;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }

    fn abilities(&mut self) -> AsmResult<AbilitySet> {
        let mut res = AbilitySet::EMPTY;
        for ab in self.list(Self::ability, "+")? {
            res = res.add(ab)
        }
        Ok(res)
    }

    fn ability(&mut self) -> AsmResult<Ability> {
        let ident = self.ident()?;
        let ab = match ident.as_str() {
            "copy" => Ability::Copy,
            "drop" => Ability::Drop,
            "key" => Ability::Key,
            "store" => Ability::Store,
            _ => {
                return Err(error(
                    self.next_loc,
                    "expected one of copy, drop, key, or store",
                ))
            },
        };
        Ok(ab)
    }

    fn type_params(
        &mut self,
        allow_phantom: bool,
    ) -> AsmResult<Vec<(Identifier, AbilitySet, bool)>> {
        if !self.is_special("<") {
            return Ok(vec![]);
        }
        self.advance()?;
        let result = self.list(
            |parser| {
                let is_phantom = if allow_phantom {
                    if parser.is_soft_kw("phantom") {
                        parser.advance()?;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                let name = parser.ident()?;
                let abs = if parser.is_special(":") {
                    parser.advance()?;
                    parser.abilities()?
                } else {
                    AbilitySet::EMPTY
                };
                Ok((name, abs, is_phantom))
            },
            ",",
        )?;
        self.expect_special(">")?;
        Ok(result)
    }

    fn visibility(&mut self) -> AsmResult<Visibility> {
        Ok(if self.is_soft_kw("public") {
            self.advance()?;
            Visibility::Public
        } else if self.is_soft_kw("friend") {
            self.advance()?;
            Visibility::Friend
        } else {
            Visibility::Private
        })
    }

    fn attributes(&mut self) -> AsmResult<Vec<FunctionAttribute>> {
        if self.is_special("#") && self.lookahead_special("[") {
            self.advance()?;
            self.advance()?;
            let attrs = self.list(
                |parser| {
                    // Helper to parse a u16 parameter after the attribute name (expects "(<num>)")
                    let parse_u16_param = |parser: &mut AsmParser| -> AsmResult<u16> {
                        parser.expect_special("(")?;
                        let result = if let Token::Number(num) = &parser.next {
                            let r = num.repr().as_u16();
                            parser.advance()?;
                            r
                        } else {
                            return Err(error(parser.next_loc, "expected number"));
                        };
                        parser.expect_special(")")?;
                        Ok(result)
                    };

                    let attr_name = parser.ident()?;
                    let attr = match attr_name.as_str() {
                        "persistent" => FunctionAttribute::Persistent,
                        "module_lock" => FunctionAttribute::ModuleLock,
                        "pack" => FunctionAttribute::Pack,
                        "unpack" => FunctionAttribute::Unpack,
                        "pack_variant" => FunctionAttribute::PackVariant(parse_u16_param(parser)?),
                        "unpack_variant" => {
                            FunctionAttribute::UnpackVariant(parse_u16_param(parser)?)
                        },
                        "test_variant" => FunctionAttribute::TestVariant(parse_u16_param(parser)?),
                        "borrow" => {
                            FunctionAttribute::BorrowFieldImmutable(parse_u16_param(parser)?)
                        },
                        "borrow_mut" => {
                            FunctionAttribute::BorrowFieldMutable(parse_u16_param(parser)?)
                        },
                        _ => {
                            return Err(error(
                                parser.next_loc,
                                format!("unknown function attribute '{}'", attr_name),
                            ))
                        },
                    };
                    Ok(attr)
                },
                ",",
            )?;
            self.expect_special("]")?;
            Ok(attrs)
        } else {
            Ok(vec![])
        }
    }

    fn decl(&mut self) -> AsmResult<Decl> {
        let loc = self.next_loc;
        let name = self.ident()?;
        self.expect_special(":")?;
        let ty = self.type_()?;
        Ok(Decl { loc, name, ty })
    }

    fn argument(&mut self) -> AsmResult<Argument> {
        if self.is_special("<") {
            self.advance()?;
            let ty = self.type_()?;
            self.expect_special(">")?;
            Ok(Argument::Type(ty))
        } else if self.is_value() && !self.lookahead_special("::") {
            let val = self.value()?;
            Ok(Argument::Constant(val))
        } else {
            let pid = self.partial_ident()?;
            let targs = self.type_args_opt()?;
            Ok(Argument::Id(pid, targs))
        }
    }

    fn is_argument(&self) -> bool {
        self.is_value() || self.is_partial_ident() || self.is_special("<")
    }

    fn instr(&mut self) -> AsmResult<Instruction> {
        let mut loc = self.next_loc;
        let mut name = self.ident()?;
        let label = if self.is_special(":") {
            self.advance()?;
            let label = Some(name);
            if self.is_tok(&Token::Newline) && self.lookahead_indent() {
                // Allow
                //  l:
                //    inst
                self.advance()?;
                self.advance()?;
            }
            loc = self.next_loc;
            name = self.ident()?;
            label
        } else {
            None
        };
        let args = if self.is_argument() {
            // Special case if first argument is type (`<t>`): in this case we do not require
            // a comma, so we can write `ld_const <T> val
            if self.is_special("<") {
                let first = self.argument()?;
                let mut args = if self.is_special(",") {
                    // We still allow a comma
                    self.advance()?;
                    self.list(Self::argument, ",")?
                } else if self.is_value() {
                    self.list(Self::argument, ",")?
                } else {
                    vec![]
                };
                args.insert(0, first);
                args
            } else {
                self.list(Self::argument, ",")?
            }
        } else {
            vec![]
        };
        // Extend loc to include args
        let loc = Loc::new(loc.start(), self.previous_loc.end());
        Ok(Instruction {
            loc,
            label,
            name,
            args,
        })
    }

    fn is_struct_or_enum(&self) -> bool {
        self.is_soft_kw("struct") || self.is_soft_kw("enum")
    }

    fn struct_or_enum(&mut self) -> AsmResult<Struct> {
        if self.is_soft_kw("struct") {
            self.advance()?;
            let (loc, name, type_params, abilities) = self.struct_header()?;
            self.expect_newline()?;
            let mut fields = vec![];
            while self.is_indent() {
                self.advance()?;
                fields.push(self.decl()?);
                self.expect_newline()?
            }
            Ok(Struct {
                loc,
                name,
                type_params,
                abilities,
                layout: StructLayout::Singleton(fields),
            })
        } else {
            self.expect_soft_kw("enum")?;
            let (loc, name, type_params, abilities) = self.struct_header()?;
            self.expect_newline()?;
            let mut variants = vec![];
            let mut cur_variant_name = None;
            let mut cur_loc = Loc::new(0, 0);
            let mut cur_fields = vec![];
            while self.is_indent() {
                self.advance()?;
                if self.is_ident() && self.lookahead_newline() {
                    // New enum variant
                    let next_loc = self.next_loc;
                    let variant_name = self.ident()?;
                    if let Some(name) = cur_variant_name {
                        variants.push((cur_loc, name, cur_fields));
                    }
                    cur_loc = next_loc;
                    cur_variant_name = Some(variant_name);
                    cur_fields = vec![]
                } else {
                    // Field for current_variant variant
                    cur_fields.push(self.decl()?)
                }
                self.expect_newline()?;
            }
            if let Some(name) = cur_variant_name {
                variants.push((cur_loc, name, cur_fields));
            }
            if variants.is_empty() {
                return Err(error(loc, "enum type must have at least one variant"));
            }
            Ok(Struct {
                loc,
                name,
                type_params,
                abilities,
                layout: StructLayout::Variants(variants),
            })
        }
    }

    fn struct_header(
        &mut self,
    ) -> AsmResult<(
        Loc,
        Identifier,
        Vec<(Identifier, AbilitySet, bool)>,
        AbilitySet,
    )> {
        let loc = self.next_loc;
        let id = self.ident()?;
        let ty_params = self.type_params(true /*allow_phantom*/)?;
        let abilities = if self.is_soft_kw("has") {
            self.advance()?;
            self.abilities()?
        } else {
            AbilitySet::EMPTY
        };
        Ok((loc, id, ty_params, abilities))
    }

    fn is_first_of_fun(&self) -> bool {
        // Notice when this is called we already checked for structs,
        // so we only check for the start token (because of modifiers
        // and attributes, we would need a deep lookahead otherwise)
        self.is_soft_kw("fun")
            || self.is_soft_kw("entry")
            || self.is_soft_kw("public")
            || self.is_soft_kw("friend")
            || self.is_soft_kw("native")
            || self.is_special("#") && self.lookahead_special("[")
    }

    fn fun(&mut self) -> AsmResult<Fun> {
        let attributes = self.attributes()?;
        let is_entry = if self.is_soft_kw("entry") {
            self.advance()?;
            true
        } else {
            false
        };
        let visibility = self.visibility()?;
        let is_native = if self.is_soft_kw("native") {
            self.advance()?;
            true
        } else {
            false
        };
        self.expect_soft_kw("fun")?;
        let loc = self.next_loc;
        let name = self.ident()?;
        let type_params = self
            .type_params(false /* allow_phantom*/)?
            .into_iter()
            .map(|(id, ab, _)| (id, ab))
            .collect();
        self.expect_special("(")?;
        let params = if self.is_ident() {
            self.list(Self::decl, ",")?
        } else {
            vec![]
        };
        self.expect_special(")")?;
        let result = if self.is_special(":") {
            self.advance()?;
            self.result_type_tuple()?
        } else {
            vec![]
        };
        let acquires = if self.is_soft_kw("acquires") {
            self.advance()?;
            self.list(Self::ident, ",")?
        } else {
            vec![]
        };
        self.expect_newline()?;
        let mut locals = vec![];
        let mut spec_clauses = vec![];
        let mut instrs = vec![];
        while self.is_indent() || self.is_ident() && self.lookahead_special(":") {
            if self.is_indent() {
                self.advance()?
            }
            if self.is_soft_kw("local") {
                if !instrs.is_empty() {
                    return Err(error(
                        self.next_loc,
                        "local declarations must precede instructions",
                    ));
                }
                self.advance()?;
                let local = self.decl()?;
                locals.push(local);
            } else if self.is_spec_clause_start() {
                if !instrs.is_empty() {
                    return Err(error(
                        self.next_loc,
                        "spec clauses must precede instructions",
                    ));
                }
                spec_clauses.push(self.spec_clause()?);
            } else {
                instrs.push(self.instr()?)
            }
            self.expect_newline()?
        }
        if is_native && (!locals.is_empty() || !instrs.is_empty()) {
            return Err(error(loc, "native function cannot have a body or locals"));
        }
        if !is_native && instrs.is_empty() {
            return Err(error(loc, "non-native function must have a body"));
        }
        Ok(Fun {
            loc,
            name,
            visibility,
            is_entry,
            is_native,
            attributes,
            type_params,
            params,
            locals,
            result,
            acquires,
            spec_clauses,
            instrs,
        })
    }

    fn module_id(
        &mut self,
        address_aliases: &BTreeMap<&IdentStr, AccountAddress>,
    ) -> AsmResult<ModuleId> {
        let addr = if matches!(&self.next, Token::Number(..)) {
            self.address()?
        } else if self.is_ident() {
            let id = self.ident()?;
            if let Some(addr) = address_aliases.get(id.as_ident_str()) {
                *addr
            } else {
                return Err(error(
                    self.next_loc,
                    format!("unknown address alias `{}`", id),
                ));
            }
        } else {
            return Err(error(self.next_loc, "expected address or address alias"));
        };
        self.expect_special("::")?;
        let id = self.ident()?;
        Ok(ModuleId::new(addr, id))
    }

    fn unit(&mut self) -> AsmResult<Unit> {
        // Skip any empty lines at beginning of file. After that, further empty lines
        // are consumed on line break.
        while self.is_tok(&Token::Newline) {
            self.advance()?
        }
        // Parse address aliases
        let mut address_aliases = vec![];
        while self.is_soft_kw("address") {
            self.advance()?;
            let name = self.ident()?;
            self.expect_special("=")?;
            let addr = self.address()?;
            self.expect_newline()?;
            address_aliases.push((name, addr));
        }
        let address_alias_map: BTreeMap<&IdentStr, AccountAddress> = address_aliases
            .iter()
            .map(|(name, addr)| (name.as_ident_str(), *addr))
            .collect();

        // Parse module header
        let name = if self.is_soft_kw("module") {
            self.advance()?;
            UnitId::Module(self.module_id(&address_alias_map)?)
        } else if self.is_soft_kw("script") {
            self.advance()?;
            UnitId::Script
        } else {
            return Err(error(self.next_loc, "expected `module` or `script` header"));
        };
        self.expect_newline()?;

        // Parse module aliases
        let mut module_aliases = vec![];
        while self.is_soft_kw("use") {
            self.advance()?;
            let module = self.module_id(&address_alias_map)?;
            let name = if self.is_soft_kw("as") {
                self.advance()?;
                self.ident()?
            } else {
                module.name.clone()
            };
            self.expect_newline()?;
            module_aliases.push((name, module));
        }

        // Parse friend modules
        let mut friend_modules = vec![];
        while self.is_soft_kw("friend") && self.lookahead_special_2("::") {
            self.advance()?;
            let module = self.module_id(&address_alias_map)?;
            self.expect_newline()?;
            friend_modules.push(module);
        }

        // Parse definitions
        let mut structs = vec![];
        let mut functions = vec![];

        while !self.is_tok(&Token::End) {
            if self.is_struct_or_enum() {
                structs.push(self.struct_or_enum()?)
            } else if self.is_first_of_fun() {
                functions.push(self.fun()?)
            } else {
                return Err(error(
                    self.next_loc,
                    "expected function or struct declaration",
                ));
            }
        }
        self.expect(&Token::End)?;
        Ok(Unit {
            name,
            address_aliases,
            module_aliases,
            friend_modules,
            structs,
            functions,
        })
    }
}

// -------------------------------------------------------------------------------------------
// Scanner

#[derive(PartialEq, Eq, Debug)]
enum Token {
    Number(U256),
    Ident(String),
    Special(String),
    Newline,
    Indent(usize),
    End,
}

fn scan(input: &[u8]) -> AsmResult<VecDeque<(Loc, Token)>> {
    let mut pos = 0;
    let end = input.len();
    let mut result = VecDeque::new();
    loop {
        // Skip space
        let start = pos;
        while pos < end && matches!(input[pos], b' ' | b'\t' | b'\r') {
            pos += 1
        }
        // Skip a line comment.  Because `/` is also specification division,
        // require `//` to start a token (beginning of input or following
        // whitespace).  Thus `x//2` is tokenized and rejected as two division
        // operators instead of silently truncating the specification clause.
        if pos + 1 < end
            && matches!(input[pos], b'/')
            && matches!(input[pos + 1], b'/')
            && (pos == 0 || input[pos - 1].is_ascii_whitespace())
        {
            pos += 2;
            // Skip until end of line
            while pos < end && !matches!(input[pos], b'\n') {
                pos += 1
            }
            continue;
        }
        // Terminate at end
        if pos >= end {
            result.push_back((loc(start..pos), Token::End));
            return Ok(result);
        }
        // Record indent
        if pos > start && (start == 0 || matches!(result.back().unwrap().1, Token::Newline)) {
            result.push_back((loc(start..pos), Token::Indent(pos - start)));
        }
        // Identify token
        let start = pos;
        let ch = input[pos] as char;
        if ch == '\n' {
            pos += 1;
            result.push_back((loc(start..pos), Token::Newline))
        } else if ch.is_ascii_digit() {
            let mut digits_start = pos;
            let mut radix = 10;
            pos += 1;
            if pos < end && matches!(input[pos], b'x' | b'X') {
                pos += 1;
                digits_start = pos;
                radix = 16
            }
            while pos < end && (input[pos] as char).is_digit(radix) {
                pos += 1
            }
            let num_str: String = from_bytes(digits_start, &input[digits_start..pos])?;
            let Ok(num) = U256::from_str_radix(&num_str, radix) else {
                return Err(error(
                    loc(start..pos),
                    format!("invalid number `{}`", num_str),
                ));
            };
            result.push_back((loc(start..pos), Token::Number(num)))
        } else if id_start(ch) {
            pos += 1;
            while pos < end && id_cont(input[pos] as char) {
                pos += 1;
            }
            result.push_back((
                loc(start..pos),
                Token::Ident(from_bytes(start, &input[start..pos])?),
            ))
        } else if special(ch) {
            pos += 1;
            if ch == ':' && pos < end && input[pos] == b':' {
                pos += 1
            }
            result.push_back((
                loc(start..pos),
                Token::Special(from_bytes(start, &input[start..pos])?),
            ));
        } else {
            return Err(error(
                loc(start..pos),
                format!("invalid character `{}`", ch),
            ));
        }
    }
}

fn from_bytes(pos: usize, b: &[u8]) -> AsmResult<String> {
    String::from_utf8(b.to_vec())
        .map_err(|_| error(loc(pos..pos + b.len()), "invalid bytes in source"))
}

fn id_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn id_cont(ch: char) -> bool {
    id_start(ch) || ch.is_ascii_digit()
}

fn special(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')'
            | '<'
            | '>'
            | '['
            | ']'
            | ','
            | ':'
            | '|'
            | '+'
            | '='
            | '&'
            | '#'
            | '-'
            | '!'
            | '*'
            | '/'
            | '%'
            | '.'
    )
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{}", n),
            Token::Ident(s) => f.write_str(s),
            Token::Special(s) => f.write_str(s),
            Token::Newline => f.write_str("<newline>"),
            Token::Indent(_) => f.write_str("<indent>"),
            Token::End => f.write_str("<end of file>"),
        }
    }
}

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
//! ```
//! unit :=
//!   { address_alias LF }
//!   ( "module" QID | "script" ) LF
//!   { struct_def | fun_def }
//!
//! struct_def :=
//!   /*TBD*/
//!
//! fun_def :=
//!   fun_modifier "fun" ID "(" [ LIST(local) ] ")" [ tuple_type ] LF
//!   { instruction LF }
//!
//! fun_modifier := [ [ "public" | "friend" ] "entry" ]
//!
//! local := ID ":" type
//!
//! type :=
//!   "|" [ LIST(type) ] "|" [ tuple_type ]  | simple_type
//!
//! tuple_type :=
//!     simple_type | "(" LIST(simple_type) ")"
//!
//! simple_type :=
//!   QID [ type_args ] | "(" type ")"
//!
//! type_args :=
//!   "<" LIST(type) ">"
//!
//! instruction :=
//!   ( INDENT | LOOKAHEAD(ID ":") )
//!   ( "local" local | opcode [ LIST(argument) ] ) LF
//!
//! opcode := IDENT # current instr set
//!
//! argument :=
//!   VALUE | QID [ type_args ] | type_args
//!

use codespan::{RawIndex, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
use move_binary_format::file_format::Visibility;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::ModuleId,
    u256::U256,
};
use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
    ops::Range,
    str::FromStr,
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

/// Represents the AST for an assembly source
#[derive(Debug)]
pub struct Unit {
    /// The name, either script or module.
    pub name: UnitId,
    /// A list of address aliases.
    pub address_aliases: Vec<(Identifier, AccountAddress)>,
    /// A list of module aliases.
    pub module_aliases: Vec<(Identifier, ModuleId)>,
    /// List of functions.
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
pub struct Fun {
    pub loc: Loc,
    pub name: Identifier,
    pub visibility: Visibility,
    pub type_params: Vec<(Identifier, AbilitySet)>,
    pub params: Vec<Local>,
    pub locals: Vec<Local>,
    pub result: Vec<Type>,
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
pub struct Local {
    pub loc: Loc,
    pub name: Identifier,
    pub ty: Type,
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
    Constant(Value),
    Id(PartialIdent, Option<Vec<Type>>),
    Type(Type),
}

#[derive(Debug)]
pub enum Value {
    Number(U256),
    Bytes(Vec<u8>),
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
    let (loc, next) = tokens.pop_front().unwrap();
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
        matches!(&self.tokens[0].1, Token::Special(s) if s == sp)
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

    fn value(&mut self) -> AsmResult<Value> {
        if let Token::Number(num) = &self.next {
            let num = *num;
            self.advance()?;
            Ok(Value::Number(num))
        } else {
            Err(error(self.next_loc, "expected value"))
        }
    }

    fn is_value(&self) -> bool {
        matches!(&self.next, Token::Number(..))
    }

    fn address(&mut self) -> AsmResult<AccountAddress> {
        if let Token::Number(num) = &self.next {
            let addr = AccountAddress::from_str(&num.to_string()).expect("valid number");
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
        if self.is_partial_ident() {
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
                self.type_tuple()?
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
        self.is_partial_ident()
    }

    fn type_list(&mut self) -> AsmResult<Vec<Type>> {
        self.list(Self::type_, ",")
    }

    fn type_tuple(&mut self) -> AsmResult<Vec<Type>> {
        if self.is_type() {
            Ok(vec![self.type_()?])
        } else if self.is_special("(") {
            self.advance()?;
            let res = self.type_list()?;
            self.expect_special(")")?;
            Ok(res)
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

    fn type_params(&mut self) -> AsmResult<Vec<(Identifier, AbilitySet)>> {
        if !self.is_special("<") {
            return Ok(vec![]);
        }
        self.advance()?;
        let result = self.list(
            |parser| {
                let name = parser.ident()?;
                let abs = if parser.is_special(":") {
                    parser.advance()?;
                    parser.abilities()?
                } else {
                    AbilitySet::EMPTY
                };
                Ok((name, abs))
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

    fn local_decl(&mut self) -> AsmResult<Local> {
        let loc = self.next_loc;
        let name = self.ident()?;
        self.expect_special(":")?;
        let ty = self.type_()?;
        Ok(Local { loc, name, ty })
    }

    fn argument(&mut self) -> AsmResult<Argument> {
        if self.is_special("<") {
            self.advance()?;
            let ty = self.type_()?;
            self.expect_special(">")?;
            Ok(Argument::Type(ty))
        } else if self.is_value() {
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
            loc = self.next_loc;
            name = self.ident()?;
            label
        } else {
            None
        };
        let args = if self.is_argument() {
            self.list(Self::argument, ",")?
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

    fn is_fun(&self) -> bool {
        self.is_soft_kw("fun") || self.is_soft_kw("public") || self.is_soft_kw("friend")
    }

    fn fun(&mut self) -> AsmResult<Fun> {
        let visibility = self.visibility()?;
        self.expect_soft_kw("fun")?;
        let loc = self.next_loc;
        let name = self.ident()?;
        let type_params = self.type_params()?;
        self.expect_special("(")?;
        let params = if self.is_ident() {
            self.list(Self::local_decl, ",")?
        } else {
            vec![]
        };
        self.expect_special(")")?;
        let result = if self.is_special(":") {
            self.advance()?;
            self.type_tuple()?
        } else {
            vec![]
        };
        self.expect_newline()?;
        let mut locals = vec![];
        let mut instrs = vec![];
        while self.is_indent() || self.is_ident() && self.lookahead_special(":") {
            if self.is_indent() {
                self.advance()?
            }
            if self.is_soft_kw("local") {
                if !instrs.is_empty() {
                    return Err(error(
                        self.next_loc,
                        "local declarations must proceed instructions",
                    ));
                }
                self.advance()?;
                let local = self.local_decl()?;
                locals.push(local);
            } else {
                instrs.push(self.instr()?)
            }
            self.expect_newline()?
        }
        Ok(Fun {
            loc,
            name,
            visibility,
            type_params,
            params,
            locals,
            result,
            instrs,
        })
    }

    fn module_id(
        &mut self,
        address_aliases: &[(Identifier, AccountAddress)],
    ) -> AsmResult<ModuleId> {
        let addr = if matches!(&self.next, Token::Number(..)) {
            self.address()?
        } else if self.is_ident() {
            let id = self.ident()?;
            if let Some((_, addr)) = address_aliases.iter().find(|(n, _)| n == &id) {
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
        let mut address_aliases = vec![];
        while self.is_soft_kw("address") {
            self.advance()?;
            let name = self.ident()?;
            self.expect_special("=")?;
            let addr = self.address()?;
            self.expect_newline()?;
            address_aliases.push((name, addr));
        }
        let mut module_aliases = vec![];
        while self.is_soft_kw("use") {
            self.advance()?;
            let module = self.module_id(&address_aliases)?;
            self.expect_soft_kw("as")?;
            let name = self.ident()?;
            self.expect_newline()?;
            module_aliases.push((name, module));
        }
        let name = if self.is_soft_kw("module") {
            self.advance()?;
            UnitId::Module(self.module_id(&address_aliases)?)
        } else if self.is_soft_kw("script") {
            UnitId::Script
        } else {
            return Err(error(self.next_loc, "expected `module` or `script` header"));
        };
        self.expect_newline()?;
        let mut functions = vec![];
        while self.is_fun() {
            functions.push(self.fun()?)
        }
        self.expect(&Token::End)?;
        Ok(Unit {
            name,
            address_aliases,
            module_aliases,
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
        let mut start = pos;
        while pos < end && matches!(input[pos], b' ' | b'\t' | b'\r') {
            pos += 1
        }
        // Skip comment
        if pos < end && matches!(input[pos], b'#') {
            pos += 1;
            // Skip until end of line
            while pos < end && !matches!(input[pos], b'\n') {
                pos += 1
            }
            start = pos;
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
    matches!(ch, '(' | ')' | '<' | '>' | ',' | ':' | '|' | '+' | '=')
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

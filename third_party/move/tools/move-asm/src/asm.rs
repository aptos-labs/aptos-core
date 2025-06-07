// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_builder::{ModuleBuilder, ModuleBuilderOptions, PartialId};
use anyhow::{anyhow, bail, Result};
use codespan::{ByteIndex, RawIndex, Span};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::{Files, Location, SimpleFile},
    term,
    term::termcolor::{Buffer, ColorChoice, StandardStream},
};
use either::Either;
use legacy_move_compiler::{
    diagnostics::{Diagnostic as LegacyDiagnostic, Diagnostics as LegacyDiagnostics},
    parser::lexer::{Lexer, Tok},
};
use move_binary_format::{
    file_format::{CompiledScript, SignatureToken},
    CompiledModule,
};
use move_command_line_common::files::FileHash;
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

pub struct Asm<'a> {
    source_name: &'a str,
    source: &'a str,
    tokens: Lexer<'a>,
    errors: Vec<Diagnostic<()>>,
    builder: ModuleBuilder<'a>,
    is_script: bool,
}

/// Currently we use as locations span's (no filename), but keep this abstracted in case
/// we want to change this.
type Loc = Span;

impl<'a> Asm<'a> {
    pub fn assemble(
        options: ModuleBuilderOptions,
        external_modules: &'a [CompiledModule],
        source_name: &'a str,
        source: &'a str,
    ) -> Result<Either<CompiledModule, CompiledScript>> {
        let builder = ModuleBuilder::new(options, external_modules);
        let tokens = Lexer::new(source, FileHash::empty());
        let mut asm = Self {
            source_name,
            source,
            tokens,
            errors: vec![],
            builder,
            is_script: false,
        };
        asm.run();
        if asm.errors.is_empty() {
            if asm.is_script {
                Ok(Either::Right(asm.builder.into_script()))
            } else {
                Ok(Either::Left(asm.builder.into_module()))
            }
        } else {
            let file = SimpleFile::new(source_name, source);
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            for diag in asm.errors {
                term::emit(&mut writer, &term::Config::default(), &file, &diag)
                    .expect("emit must not fail")
            }
            Err(anyhow!("assembler errors, aborting"))
        }
    }

    fn run(&mut self) -> Result<()> {
        let at_bol = self.tokens.beginning_of_line();
        match self.advance()? {
            (_, Tok::Script) if at_bol => {
                self.is_script = true;
            },
            (_, Tok::Module) if at_bol => {
                self.is_script = false;
                // We require a pre-pass to the source to find members defined in the module
                self.phase1()?
            },
            (loc, _) => {
                return Err(
                    self.capture_error(loc, "expected `module` or `script` at beginning of line")
                )
            },
        }
        self.phase2()
    }

    fn handle_result<T>(&mut self, loc: Loc, result: Result<T>) {
        match result {
            Ok(_) => {},
            Err(e) => self.error(loc, e.to_string()),
        }
    }

    fn phase1(&mut self) -> Result<()> {
        // We scan const, struct, and fun declaration headers and enter prototype declarations
        // into the builder.
        while (true) {
            let at_bol = self.tokens.beginning_of_line();
            let (_, tok) = self.advance()?;
            if at_bol {
                match tok {
                    Tok::Fun => {
                        let (loc, name) = self.parse_ident()?;
                        let r = self.builder.declare_fun(false, name);
                        self.handle_result(loc, r)
                    },
                    _ => { /*todo*/ },
                }
            }
        }
        Ok(())
    }

    fn phase2(&mut self) -> Result<()> {
        unimplemented!()
    }

    fn type_(&mut self) -> Result<SignatureToken> {
        let (loc, module_id_opt, id) = self.parse_qualified_id_opt()?;
        unimplemented!()
    }

    fn parse_partial_id(&mut self) -> Result<(Loc, PartialId)> {
        let start = self.tokens.start_loc();
        let opt_numerical_address = self
            .advance_if(
                |tok| tok == Tok::NumValue,
                |s, _| s.parse_numerical_address(),
            )?
            .map(|a, _| a);

        unimplemented!()
    }

    /*
    fn is_kw(&mut self, kw: impl AsRef<str>) -> bool {
        // We accept any token here as long as it matches the keyword representation.
        // Identifiers are interpreted as soft keywords in a given context
        self.tokens.content() == kw.as_ref()
    }

    fn parse_kw(&mut self, kw: impl AsRef<str>) -> anyhow::Result<Loc> {
        let str = kw.as_ref();
        if self.is_kw(str) {
            self.advance()
        } else {
            Err(self.error_and_bail(self.cur_loc(), format!("expected `{}`", str)))
        }
    }
     */

    fn parse_module_id(&mut self) -> Result<(Loc, ModuleId)> {
        let start = self.tokens.start_loc();
        let (_, address) = self.parse_address()?;
        let (_, name) = self.parse_ident()?;
        Ok((self.loc_to_end(start), ModuleId { address, name }))
    }

    fn parse_numerical_address(&mut self) -> Result<(Loc, AccountAddress)> {
        match self.peek() {
            Tok::NumValue => {
                let addr_result = AccountAddress::from_hex_literal(self.tokens.content());
                let (loc, _) = self.advance()?;
                let addr = addr_result.map_err(|_| self.capture_error(loc, "invalid address"))?;
                Ok((loc, addr))
            },
            _ => Err(self.capture_error(self.cur_loc(), "expected identifier or address")),
        }
    }

    fn parse_ident(&mut self) -> Result<(Loc, Identifier)> {
        if self.peek() == Tok::Identifier {
            let id = Identifier::new_unchecked(self.tokens.content());
            let (loc, _) = self.advance()?;
            Ok((loc, id))
        } else {
            Err(self.capture_error(self.cur_loc(), "expected identifier"))
        }
    }

    fn parse_tok(&mut self, tok: Tok) -> Result<Loc> {
        if self.peek() == tok {
            let (loc, _) = self.advance()?;
            Ok(loc)
        } else {
            Err(self.capture_error(self.cur_loc(), format!("expected `{}`", tok)))
        }
    }

    fn advance(&mut self) -> Result<(Loc, Tok)> {
        let loc = self.cur_loc();
        let tok = self.peek();
        self.tokens
            .advance()
            .map_err(|e| anyhow!("cannot read next token"))?;
        Ok((loc, tok))
    }

    fn advance_if<T>(
        &mut self,
        check: impl Fn(Tok) -> bool,
        action: impl Fn(&mut Self, usize) -> Result<T>,
    ) -> Result<Option<T>> {
        if check(self.peek()) {
            let start_loc = self.tokens.start_loc();
            action(self, start_loc).map(|t| Some(t))
        } else {
            Ok(None)
        }
    }

    fn peek(&self) -> Tok {
        self.tokens.peek()
    }

    fn loc_to_end(&self, start_loc: usize) -> Loc {
        self.loc(start_loc, self.tokens.previous_end_loc())
    }

    fn cur_loc(&self) -> Loc {
        let curr = self.tokens.previous_end_loc();
        self.loc(curr, curr)
    }

    fn loc(&self, start_loc: usize, end_loc: usize) -> Loc {
        Loc::new(
            ByteIndex(start_loc as RawIndex),
            ByteIndex(end_loc as RawIndex),
        )
    }

    fn combine_loc(&self, first: Loc, last: Loc) -> Loc {
        self.loc(first.start().to_usize(), last.end().to_usize())
    }

    fn capture_error(&mut self, loc: Loc, msg: impl AsRef<str>) -> anyhow::Error {
        self.error(loc, msg);
        anyhow!("error")
    }

    fn error(&mut self, loc: Loc, msg: impl AsRef<str>) {
        self.errors.push(
            Diagnostic::error()
                .with_message(msg.as_ref())
                .with_labels(vec![Label::primary((), loc)]),
        )
    }
}

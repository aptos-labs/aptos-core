// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{project::Project, utils::*};
use move_command_line_common::files::FileHash;
use move_core_types::account_address::*;
use move_ir_types::location::*;
use move_model::model::GlobalEnv;
use move_symbol_pool::Symbol;
use std::{
    hash::Hash,
    path::{Path, PathBuf},
    time::SystemTime,
};

// static and const var
pub static ERR_ADDRESS: once_cell::sync::Lazy<AccountAddress> =
    once_cell::sync::Lazy::new(AccountAddress::random);

// enum and struct
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddressSpace {
    Addr(AccountAddress),
    Name(Symbol),
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct FunID {
    pub(crate) addr: AccountAddress,
    pub(crate) addr_name: String,
    pub(crate) module_name: Symbol,
    pub(crate) function_name: Symbol,
}

#[derive(Debug, Clone, Default)]
pub struct SourceDefs {
    // pub(crate) sources: HashMap<PathBuf, Vec<move_compiler::parser::ast::Definition>>,
    // pub(crate) tests: HashMap<PathBuf, Vec<move_compiler::parser::ast::Definition>>,
    // pub(crate) scripts: HashMap<PathBuf, Vec<move_compiler::parser::ast::Definition>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DummyHandler;

// pub func
pub(crate) fn file_modify_time(x: &Path) -> Option<SystemTime> {
    match x.metadata() {
        Ok(x) => match x.modified() {
            Ok(x) => Some(x),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// Handler a `ItemOrAccess` producced By `Project`.
pub trait ItemOrAccessHandler: std::fmt::Display {
    fn visit_fun_or_spec_body(&self) -> bool;

    /// Visitor should finished.
    fn finished(&self) -> bool;

    // need Expr type ??
    fn need_expr_type(&self) -> bool {
        false
    }

    // current vistor handler is inlay_hints ?
    fn current_vistor_handler_is_inlay_hints(&self) -> bool {
        false
    }

    fn need_call_pair(&self) -> bool {
        false
    }

    fn handle_call_pair(&mut self, _from: FunID, _to: FunID) {}

    fn need_para_arg_pair(&self) -> bool {
        false
    }

    fn handle_project_env(
        &mut self,
        _services: &dyn HandleItemService,
        _: &GlobalEnv,
        _: &Path,
        _: String,
    ) {
    }
}

impl ItemOrAccessHandler for DummyHandler {
    fn finished(&self) -> bool {
        false
    }

    fn visit_fun_or_spec_body(&self) -> bool {
        false
    }
}

impl std::fmt::Display for DummyHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// three trait
pub trait HandleItemService: ConvertLoc {}
impl HandleItemService for Project {}

pub trait ConvertLoc {
    fn convert_file_hash_filepath(&self, hash: &FileHash) -> Option<PathBuf>;
    fn convert_loc_range(&self, loc: &Loc) -> Option<FileRange>;
}

impl_convert_loc!(Project);

impl From<Symbol> for AddressSpace {
    fn from(x: Symbol) -> Self {
        Self::Name(x)
    }
}

impl ToString for AddressSpace {
    fn to_string(&self) -> String {
        match self {
            AddressSpace::Addr(addr) => addr.to_hex_literal(),
            AddressSpace::Name(x) => x.as_str().to_string(),
        }
    }
}

impl From<AccountAddress> for AddressSpace {
    fn from(x: AccountAddress) -> Self {
        Self::Addr(x)
    }
}

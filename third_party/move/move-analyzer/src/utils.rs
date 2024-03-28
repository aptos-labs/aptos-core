// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use codespan::FileId;
use codespan_reporting::files::{Files, SimpleFiles};
use lsp_types::{Command, Location, Position};
use move_command_line_common::files::FileHash;
use move_ir_types::location::*;
use move_model::{ast::ModuleName, symbol::Symbol as SpecSymbol};
use move_package::source_package::layout::SourcePackageLayout;
use move_symbol_pool::Symbol;
use std::{collections::HashMap, path::*, vec};

/// Converts a location from the byte index format to the line/character (Position) format, where
/// line/character are 0-based.
pub fn get_loc(
    fhash: &FileHash,
    pos: ByteIndex,
    files: &SimpleFiles<Symbol, String>,
    file_id_mapping: &HashMap<FileHash, usize>,
) -> Option<Position> {
    let id = match file_id_mapping.get(fhash) {
        Some(v) => v,
        None => return None,
    };
    match files.location(*id, pos as usize) {
        Ok(v) => Some(Position {
            // we need 0-based column location
            line: v.line_number as u32 - 1,
            character: v.column_number as u32 - 1,
        }),
        Err(_) => None,
    }
}

/// Double way mapping between FileHash and FilePath.
#[derive(Debug, Default)]
pub struct PathBufHashMap {
    path_2_hash: HashMap<PathBuf, FileHash>,
    hash_2_path: HashMap<FileHash, PathBuf>,
}

impl PathBufHashMap {
    pub fn new() -> PathBufHashMap {
        Self {
            path_2_hash: HashMap::new(),
            hash_2_path: HashMap::new(),
        }
    }

    pub fn update(&mut self, path: PathBuf, hash: FileHash) {
        if let Some(hash) = self.path_2_hash.get(&path) {
            self.hash_2_path.remove(hash);
        }
        self.path_2_hash.insert(path.clone(), hash);
        self.hash_2_path.insert(hash, path);
    }

    pub(crate) fn get_path(&self, hash: &FileHash) -> Option<&'_ PathBuf> {
        self.hash_2_path.get(hash)
    }
}

#[derive(Debug, Default)]
pub struct FileLineMapping {
    m: HashMap<PathBuf /* filepath */, Vec<ByteIndex>>,
}

impl FileLineMapping {
    pub fn new() -> FileLineMapping {
        Self { m: HashMap::new() }
    }

    pub fn update(&mut self, filepath: PathBuf, content: String) {
        let mut v = vec![0];
        for (index, s) in content.as_bytes().iter().enumerate() {
            // TODO how to support windows \r\n
            if *s == 10 {
                // \n
                v.push((index + 1) as ByteIndex);
            }
        }
        if let Some(last) = content.as_bytes().last() {
            if *last != 10 {
                v.push((content.as_bytes().len()) as ByteIndex);
            }
        }
        self.m.insert(filepath, v);
    }

    pub(crate) fn translate(
        &self,
        filepath: &PathBuf,
        start_index: ByteIndex,
        mut end_index: ByteIndex,
    ) -> Option<FileRange> {
        if end_index < start_index {
            // maybe something goes wrong with syntax.rs
            // sometimes end_index < start_index.
            // this is a dummy fix.
            end_index = start_index;
        }
        let vec = self.m.get(filepath)?;
        let too_big = vec.last().map(|x| *x <= end_index).unwrap_or(false);
        if too_big {
            return None;
        }
        fn search(vec: &[ByteIndex], byte_index: ByteIndex) -> (u32, u32) {
            let mut index = bisection::bisect_left(vec, &byte_index);
            if vec[index] != byte_index {
                index -= 1;
            }
            (index as u32, byte_index - vec[index])
        }

        let (line_start, col_start) = search(&vec[..], start_index);
        let end = if let Some(t) = vec.get(line_start as usize + 1) {
            if *t > end_index {
                // Most case O(1) so we can have the same result but more fast.
                Some((line_start, end_index - vec[line_start as usize]))
            } else {
                None
            }
        } else {
            None
        };
        let (line_end, col_end) = end.unwrap_or_else(|| search(&vec[..], end_index));
        Some(FileRange {
            path: filepath.clone(),
            line_start,
            col_start,
            line_end,
            col_end,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRange {
    pub path: PathBuf,
    /// Start.
    pub line_start: u32,
    pub col_start: u32,

    /// End.
    pub line_end: u32,
    pub col_end: u32,
}

impl FileRange {
    pub fn mk_location(&self) -> lsp_types::Location {
        let range = lsp_types::Range {
            start: lsp_types::Position {
                line: self.line_start,
                character: self.col_start,
            },
            end: Position {
                line: self.line_end,
                character: self.col_end,
            },
        };
        let uri = url::Url::from_file_path(self.path.as_path()).unwrap();
        lsp_types::Location::new(uri, range)
    }
}

impl std::fmt::Display for FileRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}:({},{}):({},{})",
            self.path.as_path(),
            self.line_start,
            self.col_start,
            self.line_end,
            self.col_end
        )
    }
}

/// Path concat from
pub fn path_concat(p1: &Path, p2: &Path) -> PathBuf {
    let p2: Vec<_> = p2.components().collect();
    let is_abs = matches!(
        p2.get(0).unwrap(),
        Component::RootDir | Component::Prefix(_)
    );
    let mut p1: Vec<_> = p1.components().collect();
    normal_path_components(
        if is_abs {
            &p2
        } else {
            {
                p1.extend(p2);
                &p1
            }
        },
    )
}

/// concat Move.toml file.
pub fn path_concat_move_toml(p1: &Path, p2: &Path) -> PathBuf {
    let p1_is_move_toml = match p1.to_str() {
        Some(x) => x.ends_with(PROJECT_FILE_NAME),
        None => false,
    };
    if p1_is_move_toml {
        let mut p1 = p1.to_path_buf();
        p1.pop();
        path_concat(p1.as_path(), p2)
    } else {
        path_concat(p1, p2)
    }
}

pub fn normal_path_components(x: &Vec<Component<'_>>) -> PathBuf {
    let mut ret = PathBuf::new();
    for v in x {
        match v {
            Component::Prefix(x) => ret.push(x.as_os_str()),
            Component::RootDir => ret.push("/"),
            Component::CurDir => {},
            Component::ParentDir => {
                let _ = ret.pop();
            },
            Component::Normal(x) => ret.push(*x),
        }
    }
    if ret.to_str().unwrap() == "" {
        ret.push(".")
    }
    ret
}

pub(crate) fn normal_path(p: &Path) -> PathBuf {
    let x: Vec<_> = p.components().collect();
    normal_path_components(&x)
}

pub trait GetPosition {
    fn get_position(&self) -> (PathBuf, u32 /* line */, u32 /* col */);
    fn in_range(x: &impl GetPosition, range: &FileRange) -> bool {
        let (filepath, line, col) = x.get_position();
        if filepath != range.path.clone() {
            return false;
        }
        if line < range.line_start {
            return false;
        }
        if line == range.line_start && col < range.col_start {
            return false;
        }
        if line > range.line_end {
            return false;
        }
        if line == range.line_end && col > range.col_end {
            return false;
        }
        true
    }
}

pub struct GetPositionStruct {
    pub fpath: PathBuf,
    pub line: u32,
    pub col: u32,
}

impl GetPosition for GetPositionStruct {
    fn get_position(&self) -> (PathBuf, u32 /* line */, u32 /* col */) {
        (self.fpath.clone(), self.line, self.col)
    }
}

pub fn discover_manifest_and_kind(x: &Path) -> Option<(PathBuf, SourcePackageLayout)> {
    let mut x: Vec<_> = x.components().collect();
    // We should be able at least pop one.
    x.pop()?;
    let mut layout: Option<&SourcePackageLayout> = None;
    while !x.is_empty() {
        while !x.is_empty() {
            layout = x
                .last()
                .and_then(|x| match x.as_os_str().to_str().unwrap() {
                    "tests" => Some(&SourcePackageLayout::Tests),
                    "sources" => Some(&SourcePackageLayout::Sources),
                    "scripts" => Some(&SourcePackageLayout::Scripts),
                    _ => None,
                });
            if layout.is_some() {
                break;
            }
            x.pop();
        }
        let layout = layout?;
        // Pop tests or sources ...
        x.pop()?;
        let mut manifest_dir = PathBuf::new();
        for x in x.iter() {
            manifest_dir.push(x);
        }
        // check if manifest exists.
        let mut manifest_file = manifest_dir.clone();
        manifest_file.push(PROJECT_FILE_NAME);
        if manifest_file.exists() {
            return Some((manifest_dir, layout.clone()));
        }
    }
    None
}

pub fn is_sub_dir(p: PathBuf, mut sub: PathBuf) -> bool {
    while sub.pop() {
        if p == sub {
            return true;
        }
    }
    false
}

/// There command should implemented in `LSP` client.
pub enum MoveAnalyzerClientCommands {
    GotoDefinition(Location),
}

impl MoveAnalyzerClientCommands {
    pub(crate) fn to_lsp_command(&self) -> Command {
        match self {
            MoveAnalyzerClientCommands::GotoDefinition(x) => Command::new(
                "Goto Definition".to_string(),
                "aptos-move-analyzer.goto_definition".to_string(),
                Some(vec![serde_json::to_value(PathAndRange::from(x)).unwrap()]),
            ),
        }
    }
}

use lsp_types::Range;

#[derive(Clone, serde::Serialize)]
pub struct PathAndRange {
    range: Range,
    fpath: String,
}

impl From<&Location> for PathAndRange {
    fn from(value: &Location) -> Self {
        Self {
            range: value.range,
            fpath: value
                .uri
                .to_file_path()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        }
    }
}

pub const PROJECT_FILE_NAME: &str = "Move.toml";

use move_model::model::{GlobalEnv, ModuleId};
pub fn get_target_module(
    env: &GlobalEnv,
    move_file_path: &Path,
    target_module_id: &mut ModuleId,
) -> bool {
    let mut move_file_str: &str = "null_move_file";
    if let Some(file_stem) = move_file_path.file_stem() {
        if let Some(file_stem_str) = file_stem.to_str() {
            if file_stem_str.contains(".spec") {
                if let Some(first_part) = file_stem_str.split('.').next() {
                    move_file_str = first_part;
                }
            } else {
                move_file_str = file_stem_str;
                log::info!("move_file_str = {:?}", move_file_str)
            }
        }
    }
    for module in env.get_target_modules() {
        if module.matches_name(move_file_str) {
            *target_module_id = module.get_id();
            return true;
        }
    }
    false
}

pub fn fpath_str_is_equal(s1: &String, s2: &String) -> bool {
    if s1.len() != s2.len() {
        return false;
    }

    for (c1, c2) in s1.chars().zip(s2.chars()) {
        if c1 == c2 || (c1 == '\\' && c2 == '/') || (c1 == '/' && c2 == '\\') {
            continue;
        } else {
            return false;
        }
    }
    true
}

use move_model::model::ModuleEnv;
pub fn get_modules_by_fpath_in_target_modules<'a>(
    env: &'a GlobalEnv,
    fpath: &Path,
) -> Vec<ModuleEnv<'a>> {
    log::trace!("get target module by path");
    let mut result_vec_modules: Vec<ModuleEnv> = vec![];
    for module_env in env.get_target_modules() {
        log::info!(
            "{}",
            env.get_file(module_env.get_loc().file_id())
                .to_string_lossy()
                .to_string()
        );
        if !fpath_str_is_equal(
            &env.get_file(module_env.get_loc().file_id())
                .to_string_lossy()
                .to_string(),
            &fpath.to_string_lossy().to_string(),
        ) {
            continue;
        }
        result_vec_modules.push(module_env);
    }
    result_vec_modules
}

pub fn get_modules_by_fpath_in_all_modules<'a>(
    env: &'a GlobalEnv,
    fpath: &Path,
) -> Vec<ModuleEnv<'a>> {
    let mut result_vec_modules: Vec<ModuleEnv> = vec![];
    for module_env in env.get_modules() {
        if !fpath_str_is_equal(
            &env.get_file(module_env.get_loc().file_id())
                .to_string_lossy()
                .to_string(),
            &fpath.to_string_lossy().to_string(),
        ) {
            continue;
        }
        result_vec_modules.push(module_env);
    }
    result_vec_modules
}

pub fn get_file_id_by_fpath_in_all_modules(env: &GlobalEnv, fpath: &Path) -> Option<FileId> {
    let mut result_file_id = Default::default();
    for module_env in env.get_modules() {
        if fpath_str_is_equal(
            &env.get_file(module_env.get_loc().file_id())
                .to_string_lossy()
                .to_string(),
            &fpath.to_string_lossy().to_string(),
        ) {
            result_file_id = Some(module_env.get_loc().file_id());
        }
    }
    result_file_id
}

pub fn collect_use_decl(
    addrname_2_addrnum: &std::collections::HashMap<String, String>,
    module_env: &ModuleEnv,
    global_env: &GlobalEnv,
) -> HashMap<ModuleName, Vec<SpecSymbol>> {
    let mut result: HashMap<ModuleName, Vec<SpecSymbol>> = Default::default();
    for using_decl in module_env.get_use_decls() {
        let using_decl_name = using_decl.module_name.display_full(global_env).to_string();

        let before_after = using_decl_name.split("::").collect::<Vec<_>>();
        if before_after.len() != 2 {
            log::error!("use decl module name len should be 2");
            continue;
        }

        let addr_name = before_after[0];
        log::info!("full module name {}", using_decl_name);
        let addr_num = match addrname_2_addrnum.get(addr_name) {
            Some(x) => x,
            None => {
                log::error!("use decl module name not found in project");
                continue;
            },
        };

        let addr_addrnum_with_module_name =
            ModuleName::from_str(addr_num, using_decl.module_name.name());
        let mut using_decl_member: Vec<SpecSymbol> = vec![];
        for (_, acc_symbol, may_sym) in using_decl.members.clone().into_iter() {
            match may_sym {
                Some(x) => using_decl_member.push(x),
                None => using_decl_member.push(acc_symbol),
            }
        }
        result.insert(addr_addrnum_with_module_name, using_decl_member);
    }
    result
}

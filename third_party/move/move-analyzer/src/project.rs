// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use super::utils::*;
use crate::analyzer_handler::*;
use move_compiler::shared::PackagePaths;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    rc::Rc,
    time::SystemTime,
};

/// Project
pub struct Project {
    pub(crate) modules: HashMap<
        PathBuf, // manifest path.
        Rc<RefCell<SourceDefs>>,
    >, // This modules is consistent with the asts in MultiProject
    pub(crate) manifests: Vec<move_package::source_package::parsed_manifest::SourceManifest>,
    pub(crate) hash_file: Rc<RefCell<PathBufHashMap>>,
    pub(crate) file_line_mapping: Rc<RefCell<FileLineMapping>>,
    pub(crate) manifest_paths: Vec<PathBuf>,
    pub(crate) manifest_not_exists: HashSet<PathBuf>,
    pub(crate) manifest_load_failures: HashSet<PathBuf>,
    pub(crate) manifest_mod_time: HashMap<PathBuf, Option<SystemTime>>,
    pub(crate) global_env: move_model::model::GlobalEnv,
    pub(crate) current_modifing_file_content: String,
    pub(crate) targets: Vec<PackagePaths<std::string::String, std::string::String>>,
    pub(crate) dependents: Vec<PackagePaths<std::string::String, std::string::String>>,
    pub(crate) addrname_2_addrnum: std::collections::HashMap<String, String>,
    pub err_diags: String,
}

impl Project {
    pub fn loc_to_range(&self, loc: &move_model::model::Loc) -> lsp_types::Range {
        let location_start = self.global_env.get_location(loc).unwrap();
        let location_end = self
            .global_env
            .get_location(&move_model::model::Loc::new(
                loc.file_id(),
                codespan::Span::new(loc.span().end(), loc.span().end()),
            ))
            .unwrap();
        lsp_types::Range {
            start: lsp_types::Position {
                line: location_start.line.0,
                character: location_start.column.0,
            },
            end: lsp_types::Position {
                line: location_end.line.0,
                character: location_end.column.0,
            },
        }
    }
}

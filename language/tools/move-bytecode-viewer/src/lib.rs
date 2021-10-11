// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bytecode_viewer::BytecodeViewer, source_viewer::ModuleViewer,
    tui::tui_interface::start_tui_with_interface, viewer::Viewer,
};
use bytecode_source_map::{source_map::SourceMap, utils::source_map_from_file};
use move_binary_format::file_format::CompiledModule;
use std::{
    fs,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

pub mod bytecode_viewer;
pub mod interfaces;
pub mod source_viewer;
pub mod tui;
pub mod viewer;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Move Bytecode Explorer",
    about = "Explore Move bytecode and how the source code compiles to it"
)]
pub struct BytecodeViewerConfig {
    /// The path to the module binary
    #[structopt(long = "module-path", short = "b")]
    pub module_binary_path: PathBuf,

    /// The path to the source map for the module binary
    #[structopt(long = "source-map-path")]
    pub module_sourcemap_path: PathBuf,

    /// The path to the source file
    #[structopt(long = "source-path", short = "s")]
    pub source_file_path: PathBuf,
}

pub fn start_viewer_in_memory(
    compiled_module: CompiledModule,
    source_map: SourceMap,
    source_path: &Path,
) {
    let module_viewer = ModuleViewer::new(compiled_module.clone(), source_map.clone(), source_path);
    let bytecode_viewer = BytecodeViewer::new(source_map, &compiled_module);
    let interface = Viewer::new(module_viewer, bytecode_viewer);
    start_tui_with_interface(interface).unwrap();
}

impl BytecodeViewerConfig {
    pub fn start_viewer(&self) {
        let bytecode_bytes =
            fs::read(&self.module_binary_path).expect("Unable to read bytecode file");
        let compiled_module = CompiledModule::deserialize(&bytecode_bytes)
            .expect("Module blob can't be deserialized");

        let source_map = source_map_from_file(&self.module_sourcemap_path).unwrap();

        let source_path = Path::new(&self.source_file_path);
        start_viewer_in_memory(compiled_module, source_map, source_path)
    }
}

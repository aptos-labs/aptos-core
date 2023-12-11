use std::path::PathBuf;

use move_model::model::GlobalEnv;

mod resolved_graph;

pub mod build;
pub fn main(path: Option<PathBuf>) -> anyhow::Result<GlobalEnv> {
    build::build_ast(path)
}

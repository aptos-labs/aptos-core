use move_model::model::GlobalEnv;
use std::path::PathBuf;

mod resolved_graph;

pub mod build;
pub fn main(path: Option<PathBuf>) -> anyhow::Result<GlobalEnv> {
    build::build_ast(path)
}

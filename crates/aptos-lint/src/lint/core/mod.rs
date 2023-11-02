use move_compiler::FullyCompiledProgram;


pub mod build;
pub fn main(path: std::path::PathBuf) -> anyhow::Result<FullyCompiledProgram> {
    build::build_ast(&path)
}
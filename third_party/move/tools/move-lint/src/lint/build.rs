use anyhow::Result;
use move_model::model::GlobalEnv;
use move_package::{source_package::layout::SourcePackageLayout, BuildConfig, ModelConfig};
use std::path::{Path, PathBuf};

pub enum ModelVersion {
    V1,
    V2,
}

pub struct CompiledModel {
    pub model: GlobalEnv,
    pub version: ModelVersion,
}

fn handle_reroot_path<T, F>(path: Option<PathBuf>, f: F) -> Result<T>
where
    F: FnOnce(PathBuf) -> Result<T>,
{
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let rooted_path = SourcePackageLayout::try_find_root(&path.canonicalize()?)?;
    let pop = std::env::current_dir().unwrap();
    std::env::set_current_dir(rooted_path).unwrap();
    let ret = f(PathBuf::from("."));
    std::env::set_current_dir(pop).unwrap();
    ret
}

fn compile_ast(path: &Path) -> Result<(CompiledModel, CompiledModel)> {
    let build_config: BuildConfig = BuildConfig::default();
    let build_config_v1 = build_config.clone();

    let model_v2 = build_config
        .move_model_v2_for_package(
            path,
            ModelConfig {
                target_filter: None,
                all_files_as_targets: false,
            },
        )
        .unwrap_or_else(|e| panic!("Unable to build move model: {}", e));
    let model_v1 = build_config_v1
        .move_model_for_package(
            path,
            ModelConfig {
                target_filter: None,
                all_files_as_targets: false,
            },
        )
        .unwrap_or_else(|e| panic!("Unable to build move model: {}", e));
    Ok((
        CompiledModel {
            model: model_v1,
            version: ModelVersion::V1,
        },
        CompiledModel {
            model: model_v2,
            version: ModelVersion::V2,
        },
    ))
}

pub fn build_ast(path: Option<PathBuf>) -> Result<(CompiledModel, CompiledModel)> {
    handle_reroot_path(path, |rerooted_path| compile_ast(&rerooted_path))
}

use super::resolved_graph::TraitResolvedGraph;
use anyhow::Result;
use move_compiler::{shared::known_attributes::KnownAttribute, Flags};
use move_core_types::account_address::AccountAddress;
use move_model::{
    model::GlobalEnv, options::ModelBuilderOptions,
    run_model_builder_with_options_and_compilation_flags,
};
use move_package::{
    resolution::resolution_graph::{ResolutionGraph, ResolvedGraph},
    source_package::layout::SourcePackageLayout,
    BuildConfig,
};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

trait TraitAstConfig {
    fn get_resolution_graph<W: Write>(
        self,
        path: &Path,
        writer: &mut W,
    ) -> Result<ResolutionGraph<AccountAddress>>;
}

impl TraitAstConfig for BuildConfig {
    fn get_resolution_graph<W: Write>(
        self,
        path: &Path,
        writer: &mut W,
    ) -> Result<ResolutionGraph<AccountAddress>> {
        let resolution_graph = self.resolution_graph_for_package(path, writer)?;

        Ok(resolution_graph)
    }
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

pub fn compile_ast(path: &Path) -> Result<GlobalEnv> {
    let build_config: BuildConfig = BuildConfig::default();
    let resolution_graph: ResolvedGraph =
        build_config.get_resolution_graph(path, &mut std::io::stdout())?;
    resolution_graph.check_cyclic_dependency()?;
    let (sources_package_paths, deps_package_paths) = resolution_graph.get_root_package_paths()?;

    let model = run_model_builder_with_options_and_compilation_flags(
        vec![sources_package_paths],
        deps_package_paths,
        ModelBuilderOptions {
            compile_via_model: true,
            ..ModelBuilderOptions::default()
        },
        Flags::empty(),
        KnownAttribute::get_all_attribute_names(),
    )
    .unwrap_or_else(|e| panic!("Unable to build move model: {}", e));
    Ok(model)
}

pub fn build_ast(path: Option<PathBuf>) -> Result<GlobalEnv> {
    handle_reroot_path(path, |rerooted_path| compile_ast(&rerooted_path))
}

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This script calculates the number of transitive dependencies and the total
//! sizes of dependencies for modules.
//!   - Modules should be in the form of binary files with a `.mv` extension,
//!     located in the specified input directory.
//!   - Output is saved to a file named `modules.csv`.

use anyhow::{format_err, Result};
use clap::Parser;
use move_binary_format::{access::ModuleAccess, CompiledModule};
use move_core_types::language_storage::ModuleId;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    cmp::Ordering,
    collections::{btree_map, BTreeMap},
    fmt::Write,
    path::PathBuf,
};
use tokio::fs;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the module directory
    #[clap(long, value_parser)]
    path: String,
}

async fn list_files_with_extension(
    dir: &str,
    extension: &str,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = vec![];
    let mut stack = vec![PathBuf::from(dir)];

    while let Some(curr_dir) = stack.pop() {
        let mut entries = fs::read_dir(curr_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == extension) {
                paths.push(path);
            } else if path.is_dir() {
                stack.push(path);
            }
        }
    }

    Ok(paths)
}

async fn read_modules(dir: &str) -> Result<Vec<Vec<u8>>> {
    let paths = list_files_with_extension(dir, "mv").await?;

    let reads = paths
        .into_iter()
        .map(|path| async move { fs::read(path).await });

    futures::future::join_all(reads)
        .await
        .into_iter()
        .map(|res| res.map_err(|_e| format_err!("failed to read file")))
        .collect()
}

#[derive(Debug)]
struct ModuleInfo {
    size: usize,
    deps: Vec<ModuleId>,
    friends: Vec<ModuleId>,
}

fn extract_module_info_single(bytes: &[u8]) -> Result<(ModuleId, ModuleInfo)> {
    let module = CompiledModule::deserialize(bytes)?;

    Ok((module.self_id().clone(), ModuleInfo {
        size: bytes.len(),
        deps: module.immediate_dependencies(),
        friends: module.immediate_friends(),
    }))
}

fn extract_module_info(modules: &Vec<Vec<u8>>) -> Result<BTreeMap<ModuleId, ModuleInfo>> {
    Ok(rayon::scope(move |_s| {
        modules
            .par_iter()
            .flat_map(|bytes| extract_module_info_single(bytes).ok())
    })
    .collect())
}

#[derive(Debug)]
struct DepInfo {
    count_regular: usize,
    size_regular: usize,
    count_system: usize,
    size_system: usize,
}

fn compute_dependency_size_single<'a>(
    modules: &'a BTreeMap<ModuleId, ModuleInfo>,
    entrypoint: &'a ModuleId,
) -> DepInfo {
    let mut stack = vec![entrypoint];
    let mut visited = BTreeMap::new();
    visited.insert(entrypoint, ());

    let mut info = DepInfo {
        count_regular: 0,
        size_regular: 0,
        count_system: 0,
        size_system: 0,
    };

    macro_rules! explore {
        ($module_id:expr) => {
            match visited.entry($module_id) {
                btree_map::Entry::Occupied(_entry) => (),
                btree_map::Entry::Vacant(entry) => {
                    stack.push($module_id);
                    entry.insert(());
                },
            }
        };
    }

    explore!(entrypoint);

    while let Some(module_id) = stack.pop() {
        let module_info = modules.get(module_id).expect("missing module info");

        let addr = module_id.address();
        if addr.is_special() {
            info.count_system += 1;
            info.size_system += module_info.size;
        } else {
            info.count_regular += 1;
            info.size_regular += module_info.size;
        }

        for dep in &module_info.deps {
            explore!(dep);
        }
        for friend in &module_info.friends {
            explore!(friend);
        }
    }

    info
}

fn compute_dependency_sizes(modules: &BTreeMap<ModuleId, ModuleInfo>) -> Vec<(&ModuleId, DepInfo)> {
    // Note: switch to a strongly-connected-component-based algorithm to avoid duplicate work.
    modules
        .par_iter()
        .map(|(module_id, _)| {
            (
                module_id,
                compute_dependency_size_single(modules, module_id),
            )
        })
        .collect()
}

fn render_data_csv(info: &[(&ModuleId, DepInfo)]) -> Result<String> {
    let mut s = String::new();
    writeln!(
        s,
        "module,\"#deps (user)\",\"total size in bytes (user)\",\"#deps (system)\",\"total size in bytes (system)\""
    )?;

    for (module_id, info) in info {
        writeln!(
            s,
            "{},{},{},{},{}",
            module_id, info.count_regular, info.size_regular, info.count_system, info.size_system
        )?;
    }

    Ok(s)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let modules = read_modules(&args.path).await?;
    println!("Read {} modules", modules.len());

    let info = extract_module_info(&modules)?;
    println!(
        "Deserialized {} out of {} modules",
        info.len(),
        modules.len()
    );

    let mut deps = compute_dependency_sizes(&info);

    deps.sort_by(|(left_id, left), (right_id, right)| {
        match right.count_regular.cmp(&left.count_regular) {
            Ordering::Equal => (),
            ord => return ord,
        }
        match right.size_regular.cmp(&left.size_regular) {
            Ordering::Equal => (),
            ord => return ord,
        }
        left_id.cmp(right_id)
    });

    let csv = render_data_csv(&deps)?;
    fs::write("modules.csv", csv).await?;

    Ok(())
}

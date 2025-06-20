// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use clap::Parser;
use move_binary_format::CompiledModule;
use move_core_types::language_storage::ModuleId;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{fmt::Write, path::PathBuf};
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

#[derive(Debug, Clone)]
struct ModuleInfo {
    size: usize,
    expansion_size: u64,
}

fn extract_module_info_single(bytes: &[u8]) -> Result<(ModuleId, ModuleInfo)> {
    let res = CompiledModule::deserialize(bytes);
    let module = res?;

    let expansion_size =
        move_binary_format::check_complexity::check_module_complexity(&module, u64::MAX).unwrap();

    Ok((module.self_id().clone(), ModuleInfo {
        size: bytes.len(),
        expansion_size,
    }))
}

fn extract_module_info(modules: &Vec<Vec<u8>>) -> Result<Vec<(ModuleId, ModuleInfo)>> {
    Ok(rayon::scope(move |_s| {
        modules
            .par_iter()
            .flat_map(|bytes| extract_module_info_single(bytes).ok())
    })
    .collect())
}

fn render_data_csv<'a>(
    info: impl IntoIterator<Item = (&'a ModuleId, &'a ModuleInfo)>,
) -> Result<String> {
    let mut s = String::new();
    writeln!(s, "module,size,\"expansion size\"")?;

    for (module_id, info) in info.into_iter() {
        writeln!(s, "{},{},{}", module_id, info.size, info.expansion_size)?;
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

    let csv = render_data_csv(info.iter().map(|(id, info)| (id, info)))?;
    fs::write("modules.csv", csv).await?;

    Ok(())
}

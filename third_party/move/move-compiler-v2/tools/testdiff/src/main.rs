// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A tool to create an inventory of the differences between the v1 and the v2 test suites. This
//! is used to track the completeness of test porting

use clap::Parser;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    path::PathBuf,
};
use walkdir::WalkDir;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
struct Options {
    /// Print unmatched tests. If no other option is used, this is the default.
    #[clap(short = 'u')]
    print_unmatched: bool,

    /// Print matched tests.
    #[clap(short = 'm')]
    print_matched: bool,

    /// Print new tests.
    #[clap(short = 'n')]
    print_new: bool,

    /// Diff against e2e (transactional) tests instead of unit tests.
    #[clap(short = 'e')]
    e2e_tests: bool,
}

fn main() {
    let mut opts = Options::parse();
    if !opts.print_new && !opts.print_matched && !opts.print_unmatched {
        opts.print_unmatched = true;
    }
    match run(opts) {
        Ok(_) => {},
        Err(e) => println!("ERROR: {}", e),
    }
}

const V1_UNIT_ROOT: &str = "move-compiler/tests/move_check";
const V2_UNIT_ROOT: &str = "move-compiler-v2/tests";
const V1_E2E_ROOT: &str = "move-compiler/transactional-tests/tests";
const V2_E2E_ROOT: &str = "move-compiler-v2/transactional-tests/tests";

/// Ignored directories in the move_check tree.
static IGNORED_UNIT_PATHS: Lazy<BTreeSet<String>> = Lazy::new(|| {
    let mut set = BTreeSet::new();
    set.insert("parser".to_string());
    set.insert("expansion".to_string());
    set.insert("unit_test".to_string());
    set.insert("skip_attribute_check".to_string());
    set.insert("evm".to_string());
    set.insert("async".to_string());
    set.insert("dependencies".to_string());
    set
});

/// Remapping of test directories between v2 and v1 tree. This are paths
/// relative to the according roots.
static UNIT_PATH_REMAP: Lazy<BTreeMap<String, String>> = Lazy::new(|| {
    let mut map = BTreeMap::new();
    map.insert(
        "reference-safety/v1-tests".to_string(),
        "borrows".to_string(),
    );
    map.insert("friends/v1-tests".to_string(), "naming".to_string());
    map.insert(
        "uninit-use-checker/v1-commands".to_string(),
        "commands".to_string(),
    );
    map.insert(
        "uninit-use-checker/v1-locals".to_string(),
        "locals".to_string(),
    );
    map.insert(
        "uninit-use-checker/v1-borrows".to_string(),
        "borrows".to_string(),
    );
    map.insert(
        "uninit-use-checker/v1-borrow-tests".to_string(),
        "borrow_tests".to_string(),
    );
    map
});

/// Represents a key how to identify a test. At this time, a unique key is given by the
/// Move source file name and its parent directory in the v1 tree
/// (e.g.`borrows/borrow_global.move`).
#[derive(Clone, PartialOrd, Ord, PartialEq, Eq)]
struct TestKey {
    parent_name: String,
    file_name: String,
}

impl TestKey {
    fn new(parent_name: impl ToString, file_name: impl ToString) -> Self {
        TestKey {
            parent_name: parent_name.to_string(),
            file_name: file_name.to_string(),
        }
    }
}

impl Display for TestKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.parent_name, self.file_name)
    }
}

fn run(opts: Options) -> anyhow::Result<()> {
    let (v1_root, v2_root) = if opts.e2e_tests {
        (V1_E2E_ROOT, V2_E2E_ROOT)
    } else {
        (V1_UNIT_ROOT, V2_UNIT_ROOT)
    };
    let v1 = classify_tests(&opts, collect_tests(v1_root)?, false);
    let mut v2 = classify_tests(&opts, collect_tests(v2_root)?, true);
    let mut matched = vec![];
    let mut missed = BTreeMap::new();
    for (key, mut v1_path) in v1 {
        if let Some(mut v2_path) = v2.remove(&key) {
            v1_path.set_extension("exp");
            v2_path.set_extension("exp");
            matched.push((key, v1_path, v2_path))
        } else {
            missed.insert(key, v1_path);
        }
    }
    if opts.print_unmatched {
        let mut missed_by_parent: BTreeMap<String, BTreeMap<_, _>> = BTreeMap::new();
        for (key, path) in &missed {
            missed_by_parent
                .entry(key.parent_name.clone())
                .or_default()
                .insert(key.file_name.clone(), path.clone());
        }
        for (parent, files) in missed_by_parent {
            println!("{}/{}/{{", v1_root, parent);
            for (file, _) in files {
                println!("  {},", file);
            }
            println!("}}");
        }
    }
    if opts.print_matched {
        for (_, p1, p2) in &matched {
            println!("{}   {}", p1.display(), p2.display())
        }
    }
    if opts.print_new {
        for p in v2.values() {
            println!("{}", p.display())
        }
    }
    // Send statistics to stderr so one can pipe the data.
    eprintln!(
        "matched {} tests, missed {}, v2 has {} new tests",
        matched.len(),
        missed.len(),
        v2.len()
    );
    Ok(())
}

fn collect_tests(root: &str) -> anyhow::Result<BTreeSet<PathBuf>> {
    let mut result = BTreeSet::new();
    for entry in WalkDir::new(root) {
        let path = entry?.into_path();
        if path.display().to_string().ends_with(".move") {
            result.insert(path);
        }
    }
    Ok(result)
}

fn classify_tests(
    opts: &Options,
    tests: BTreeSet<PathBuf>,
    for_v2: bool,
) -> BTreeMap<TestKey, PathBuf> {
    let mut result = BTreeMap::new();
    for test in tests {
        let Some(mut parent) = test.parent().map(|p| p.to_owned()) else {
            continue;
        };
        let mut parent_str = parent.display().to_string();
        if !opts.e2e_tests && for_v2 {
            for (a, b) in &*UNIT_PATH_REMAP {
                if parent_str.contains(a) {
                    parent_str = parent_str.replace(a, b);
                    parent = PathBuf::from(parent_str.clone());
                    break;
                }
            }
        }
        if !opts.e2e_tests && IGNORED_UNIT_PATHS.iter().any(|p| parent_str.contains(p)) {
            continue;
        }
        if let (Some(file_name), Some(parent_name)) = (test.file_name(), parent.file_name()) {
            let key = TestKey::new(parent_name.to_string_lossy(), file_name.to_string_lossy());
            if let Some(old) = result.insert(key.clone(), test.clone()) {
                warn(format!(
                    "test `{}` and `{}` share common key `{}`, discarding former one",
                    old.display(),
                    test.display(),
                    key
                ))
            }
        }
    }
    result
}

fn warn(msg: impl AsRef<str>) {
    println!("WARNING: {}", msg.as_ref())
}

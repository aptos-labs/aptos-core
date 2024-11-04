// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A tool to create an inventory of the differences between the v1 and the v2 test suites. This
//! is used to track the completeness of test porting

use clap::Parser;
use once_cell::sync::Lazy;
use regex::{self, Regex};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    path::PathBuf,
};
use walkdir::WalkDir;

// This can be set manually to true when debugging extensions to this tool.
const DEBUG: bool = false;

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
    set.insert("evm".to_string());
    set.insert("async".to_string());
    set
});

/// Remapping of test directories between v2 and v1 tree. These are paths
/// relative to the according roots.
static UNIT_PATH_REMAP: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    vec![
        ("reference-safety/v1-tests", "borrows"),
        ("reference-safety/v1-borrow-tests", "borrow_tests"),
        ("reference-safety/v1-locals", "locals"),
        ("reference-safety/v1-typing", "typing"),
        ("friends/v1-tests", "naming"),
        ("uninit-use-checker/v1-commands", "commands"),
        ("uninit-use-checker/v1-locals", "locals"),
        ("uninit-use-checker/v1-borrows", "borrows"),
        ("uninit-use-checker/v1-borrow-tests", "borrow_tests"),
        ("bytecode-generator/v1-commands", "commands"),
        ("bytecode-generator/v1-typing", "typing"),
        ("typing/v1-commands", "commands"),
        ("typing/v1-naming", "naming"),
        ("typing/v1-operators", "operators"),
        ("typing/v1-signer", "signer"),
        ("typing/v1-examples", "examples"),
        ("live-var/v1-commands", "commands"),
        ("live-var", "liveness"),
        ("visibility-checker/v1-typing", "typing"),
        ("visibility-checker/v1-naming", "naming"),
        (
            "cyclic-instantiation-checker/v1-tests",
            "instantiation_loops",
        ),
        ("cyclic-instantiation-checker/v1-typing", "typing"),
        ("abilities/v1", "typing"),
        ("acquires-checker/v1-tests", "typing"),
        ("acquires-checker/v1-borrow-tests", "borrow_tests"),
        ("attributes", "parser"),
        ("unit_test/notest", "unit_test"),
        // Map file v2../unit_test/test/foo.exp
        //   to file v1../unit_test/foo.unit_test.exp.
        ("unit_test/test", "unit_test.unit_test"),
        ("unused-assignment/v1-locals", "locals"),
        ("unused-assignment/v1-liveness", "liveness"),
        ("unused-assignment/v1-commands", "commands"),
        ("checking-lang-v1/v1-typing", "typing"),
        ("checking-lang-v1", "parser"),
        ("ability-check/v1-typing", "typing"),
        ("ability-check/v1-signer", "signer"),
        ("ability-check/v1-borrow-tests", "commands"),
        ("ability-check/v1-locals", "locals"),
        ("verification/noverify", "verification"),
        // Map file v2../verification/verify/foo.exp
        //   to file v1../verification/foo.verification.exp.
        ("verification/verify", "verification.verification"),
    ]
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
            if DEBUG {
                eprintln!(
                    "{} matches {} with key `{}`",
                    v1_path.display(),
                    v2_path.display(),
                    key
                );
            };
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
        let path_str = path.display().to_string();
        if path_str.ends_with(".move") {
            result.insert(path);
        } else if path_str.ends_with(".unit_test") || path_str.ends_with(".verification") {
            result.insert(PathBuf::from(format!("{}.{}", path_str, "move")));
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
    let split_match = Regex::new("^([^.]*)\\.([^.]*)$").unwrap();
    for test in tests {
        let Some(mut parent) = test.parent().map(|p| p.to_owned()) else {
            continue;
        };
        let Some(file_name) = test.file_name() else {
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
        if let Some(parent_name) = parent.file_name() {
            let parent_str = parent_name.to_string_lossy().to_string();
            let (parent_str, file_name_str) =
                if let Some(captures) = split_match.captures(&parent_str) {
                    // Treat a parent_str="base.ext"  as a directive
                    // to map a file "foo.move" to "foo.unit_test.move"
                    // with new_parent="base".
                    // See "unit_test.unit_test" for an example.
                    let base = captures.get(1).unwrap().as_str();
                    let ext = captures.get(2).unwrap().as_str();
                    let new_parent = base.to_string();
                    let mut new_file_name = PathBuf::from(file_name);
                    new_file_name.set_extension(ext); // replace .move with .ext
                    let file_with_ext_move = new_file_name.to_string_lossy() + ".move";
                    if DEBUG {
                        eprintln!(
                            "Parent {} matches, new_parent={}, file_with_ext={}",
                            parent_str, new_parent, file_with_ext_move
                        );
                    }
                    (new_parent, file_with_ext_move.to_string())
                } else {
                    (parent_str, file_name.to_string_lossy().to_string())
                };
            let key = TestKey::new(parent_str, file_name_str);
            if DEBUG {
                eprintln!(
                    "Adding {} mapping {} to {}",
                    if for_v2 { "v2" } else { "v1" },
                    key,
                    test.display()
                );
            }
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

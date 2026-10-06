// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Specification inference relies on the Boogie templates' models of a few
//! natives: their `$`-spec functions as exact values, and procedures which do
//! not abort as abort-free calls. These tests keep the lists in `well_known`
//! in agreement with the templates.

use move_model::well_known::{NON_ABORTING_PRELUDE_NATIVES, PRELUDE_SPEC_NATIVES};
use std::path::Path;

fn templates() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prelude = root.join("../../third_party/move/move-prover/boogie-backend/src/prelude");
    [
        prelude.join("prelude.bpl"),
        prelude.join("native.bpl"),
        root.join("src/aptos-natives.bpl"),
    ]
    .iter()
    .map(|path| {
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e))
    })
    .collect::<Vec<_>>()
    .join("\n")
}

#[test]
fn prelude_spec_natives_have_spec_functions() {
    let templates = templates();
    for (module, functions) in PRELUDE_SPEC_NATIVES {
        for function in *functions {
            let name = format!("$1.{}.${}", module, function);
            assert!(
                templates
                    .lines()
                    .any(|line| line.starts_with("function") && line.contains(&name)),
                "no Boogie function `{}` for the spec native `{}::{}`",
                name,
                module,
                function
            );
        }
    }
}

#[test]
fn non_aborting_prelude_natives_do_not_abort() {
    let templates = templates();
    let lines: Vec<&str> = templates.lines().collect();
    for (module, functions) in NON_ABORTING_PRELUDE_NATIVES {
        for function in *functions {
            let name = format!("$1.{}.{}", module, function);
            let mut procedures = 0;
            for (start, line) in lines.iter().enumerate() {
                let declares = line.starts_with("procedure")
                    && line
                        .split(['(', '\'', '{'])
                        .any(|part| part.ends_with(&name));
                if !declares {
                    continue;
                }
                procedures += 1;
                // A declaration without a body sets `$abort_flag` only if it
                // may modify it; a body must not abort.
                let aborts = if line.trim_end().ends_with(';') {
                    lines[start + 1..]
                        .iter()
                        .take_while(|line| {
                            let line = line.trim_start();
                            ["ensures", "requires", "modifies", "free"]
                                .iter()
                                .any(|clause| line.starts_with(clause))
                        })
                        .any(|line| line.trim_start().starts_with("modifies"))
                } else {
                    lines[start..]
                        .iter()
                        .take_while(|line| **line != "}")
                        .any(|line| line.contains("$Abort") || line.contains("$ExecFailureAbort"))
                };
                assert!(
                    !aborts,
                    "the procedure of `{}::{}` may abort",
                    module, function
                );
            }
            assert!(
                procedures > 0,
                "no Boogie procedure for `{}::{}`",
                module,
                function
            );
        }
    }
}

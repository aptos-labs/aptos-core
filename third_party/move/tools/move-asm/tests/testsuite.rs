// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{files::SimpleFile, term, term::termcolor::Buffer};
use either::Either;
use move_asm::{assemble, Options};
use move_binary_format::binary_views::BinaryIndexedView;
use move_command_line_common::files::FileHash;
use move_disassembler::disassembler::Disassembler;
use move_prover_test_utils::baseline_test;
use std::{fs, path::Path};

pub const EXP_EXT: &str = "exp";

datatest_stable::harness!(test_runner, "tests", r".*\.masm$");

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let options = Options::default();
    let input = fs::read_to_string(path)?;

    let mut output = String::new();
    match assemble(&options, &input) {
        Ok(result) => {
            let view = match &result {
                Either::Left(c) => BinaryIndexedView::Module(c),
                Either::Right(s) => BinaryIndexedView::Script(s),
            };
            let loc = move_ir_types::location::Loc::new(FileHash::new(&input), 0, 0);
            let disasm = Disassembler::from_view(view, loc)
                .expect("create disassembler")
                .disassemble()
                .expect("disassemble successful");
            output.push_str(&disasm)
        },
        Err(diags) => {
            let diag_file = SimpleFile::new(path.display().to_string(), &input);
            let mut error_writer = Buffer::no_color();
            for diag in diags {
                term::emit(
                    &mut error_writer,
                    &term::Config::default(),
                    &diag_file,
                    &diag,
                )
                .unwrap_or_else(|_| eprintln!("failed to print diagnostics"))
            }
            output.push_str(&format!(
                "--- Aborting with assembler errors:\n{}\n",
                String::from_utf8_lossy(&error_writer.into_inner())
            ));
        },
    }
    // Generate/check baseline.
    let baseline_path = path.with_extension(EXP_EXT);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &output)?;
    Ok(())
}

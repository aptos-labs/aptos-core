// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! masm-smith generates random Move assembly (`.masm`) transactional tests for fuzzing the Move
//! VM and the bytecode verifier.
//!
//! [`generate()`] turns fuzzer input bytes and a [`Config`] into a [`MasmTest`], and
//! [`MasmTest::render`] prints it as `.masm` text. With the default config every test passes the
//! verifier; the config can instead ask for tests that break chosen verifier rules. A few rules
//! can only be broken in compiled bytecode, which [`MasmTest::post_process_module`] does after
//! assembly.

pub mod abilities;
pub mod borrows;
pub mod config;
pub mod error;
pub mod generate;
mod in_place;
pub mod ir;
pub mod mutate;
pub(crate) mod recursion;
pub mod render;
pub mod rewrite;
pub mod splice;
pub mod state;
pub mod types;

mod blocks;
mod functions;
pub(crate) mod instructions;
mod natives;
mod struct_api;
mod structs;
mod violations;

pub use config::{Config, DimensionFlag};
pub use error::{Error, Result};
pub use generate::generate;
pub use ir::{InitDirective, MasmTest, Module, PostProcessIntent, PostProcessRecipe, Step};

/// Assembler options for generated text. Targets the newest bytecode version so that
/// version-gated verifier passes (e.g. the struct API checker) run.
pub fn assembler_options() -> move_asm::assembler::Options {
    let mut options = move_asm::assembler::Options::default();
    options.module_builder_options.bytecode_version =
        move_binary_format::file_format_common::VERSION_MAX;
    options
}

/// The stdlib the transactional test runner publishes at `0x1`, including its test-only modules
/// such as `cmp` and `mem`. Seeds the assembler context so generated code can call stdlib
/// functions.
pub fn stdlib_modules() -> &'static [move_binary_format::file_format::CompiledModule] {
    static STDLIB: std::sync::LazyLock<Vec<move_binary_format::file_format::CompiledModule>> =
        std::sync::LazyLock::new(|| {
            move_transactional_test_runner::vm_test_harness::precompiled_v2_stdlib()
                .get_pre_compiled_modules()
                .into_iter()
                .map(|module| module.named_module.module.clone())
                .collect()
        });
    &STDLIB
}

/// Split rendered `.masm` text into per-`//# publish` / per-`//# run` (script) bodies so each
/// can be handed to `move_asm::assemble()` independently. Other directives (`init`, direct
/// `//# run <qid>` calls) carry no inline body and are skipped.
pub fn split_assembler_sections(text: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current: Option<String> = None;
    fn flush(current: &mut Option<String>, out: &mut Vec<String>) {
        match current.take() {
            Some(prev) if !prev.trim().is_empty() => out.push(prev),
            _ => {},
        }
    }
    for line in text.lines() {
        if let Some(stripped) = line.strip_prefix("//# ") {
            flush(&mut current, &mut sections);
            let cmd = stripped.split_whitespace().next().unwrap_or("");
            // A `run` whose first argument is a flag (or absent) runs an inline script; one that
            // names a function (`run 0x42::m::f ...`) has no body.
            let is_inline_body = match cmd {
                "publish" => true,
                "run" => stripped
                    .split_whitespace()
                    .nth(1)
                    .is_none_or(|arg| arg.starts_with("--")),
                _ => false,
            };
            if is_inline_body {
                current = Some(String::new());
            }
        } else if let Some(buf) = current.as_mut() {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    flush(&mut current, &mut sections);
    sections
}

impl<'a> arbitrary::Arbitrary<'a> for MasmTest {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        generate(u, &Config::default()).map_err(|err| match err {
            Error::OutOfInput => arbitrary::Error::NotEnoughData,
            _ => arbitrary::Error::IncorrectFormat,
        })
    }
}

/// Returns `len` deterministic pseudorandom bytes from SplitMix64 initialized with `seed`.
pub fn seeded_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed;
    let mut bytes = Vec::with_capacity(len + 8);
    while bytes.len() < len {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^= mixed >> 31;
        bytes.extend_from_slice(&mixed.to_le_bytes());
    }
    bytes.truncate(len);
    bytes
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the Move CLI.
//!
//! Each submodule corresponds to a CLI command and contains one or more test
//! cases with `.exp` baseline files for deterministic output comparison.

mod common;

// ── Local command tests ──
mod build_publish_payload;
mod clean;
mod compile;
mod compile_script;
mod decompile;
mod disassemble;
mod document;
mod fmt_cmd;
mod init;
mod lint;
mod prove;
mod show;
mod test_cmd;

// ── Network command tests (mock-based) ──
mod publish;
mod run_function;
mod view;

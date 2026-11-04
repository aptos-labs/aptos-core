// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Selects the memory-check tier from `MONO_MOVE_MEM_CHECKS`.
//!
//! This is deliberately an environment variable read by a build script and
//! not a cargo feature: features are additive across the dependency graph, so
//! any crate that enabled one would silently harden every binary built with
//! it. Only the process that invokes `cargo` can set this.
//!
//!   unset / "none"  -> no check code is compiled (production)
//!   "hardened"      -> O(1) per-access checks
//!   "exhaustive"    -> hardened plus the expensive checks

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=MONO_MOVE_MEM_CHECKS");
    println!(
        "cargo:rustc-check-cfg=cfg(mono_move_mem_checks, values(\"hardened\", \"exhaustive\"))"
    );
    match env::var("MONO_MOVE_MEM_CHECKS").as_deref() {
        Err(_) | Ok("") | Ok("none") => {},
        Ok("hardened") => println!("cargo:rustc-cfg=mono_move_mem_checks=\"hardened\""),
        Ok("exhaustive") => println!("cargo:rustc-cfg=mono_move_mem_checks=\"exhaustive\""),
        Ok(other) => panic!(
            "MONO_MOVE_MEM_CHECKS must be unset, \"none\", \"hardened\" or \"exhaustive\"; got {other:?}"
        ),
    }
}

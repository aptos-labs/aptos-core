// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compiles the vendored fixed-point FN-DSA C implementation.

const SOURCES: &[&str] = &[
    "codec.c",
    "codec_mrm.c",
    "mq.c",
    "sha3.c",
    "sysrng.c",
    "util.c",
    "util_mrm.c",
    "kgen.c",
    "kgen_fxp.c",
    "kgen_gauss.c",
    "kgen_mp31.c",
    "kgen_ntru.c",
    "kgen_poly.c",
    "kgen_zint31.c",
    "sign.c",
    "sign_core.c",
    "sign_ext.c",
    "sign_mq2.c",
    "sign_mrm.c",
    "sign_rx.c",
    "sign_sampler.c",
    "sign_xpoly.c",
    "vrfy.c",
    "vrfy_mrm.c",
];

fn main() {
    let dir = "c-fn-dsa-fx";
    let mut build = cc::Build::new();
    build
        .include(dir)
        // Always optimize: unoptimized signing is too slow even for debug builds and tests.
        .opt_level(3)
        .warnings(false);
    for source in SOURCES {
        let path = format!("{}/{}", dir, source);
        println!("cargo:rerun-if-changed={}", path);
        build.file(path);
    }
    println!("cargo:rerun-if-changed={}/fndsa.h", dir);
    println!("cargo:rerun-if-changed={}/inner.h", dir);
    build.compile("fndsa_fx");
}

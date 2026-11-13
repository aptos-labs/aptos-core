// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A closure passed to an opaque function must stay within the frame the callee declares for
// the parameter, judged by what the closure's body does rather than by its specification.

module 0x42::fun_arg_frame_check {
    struct R has key { value: u64 }
    struct S has key { value: u64 }

    fun bump(a: address) acquires R {
        R[a].value = R[a].value + 1;
    }

    fun bump_with_spec(a: address) acquires R {
        R[a].value = R[a].value + 1;
    }
    spec bump_with_spec {
        modifies R[a];
    }

    fun read_s(a: address): u64 acquires S {
        S[a].value
    }

    fun apply_pure(fv: |address| has drop, a: address) {
        fv(a)
    }
    spec apply_pure {
        pragma opaque;
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }

    fun apply_writes_r(fv: |address| has drop, a: address) {
        fv(a)
    }
    spec apply_writes_r {
        pragma opaque;
        modifies_of<fv>(x: address) R[x];
    }

    fun apply_reads_r(fv: |address| u64 has drop, a: address): u64 {
        fv(a)
    }
    spec apply_reads_r {
        pragma opaque;
        reads_of<fv> R;
    }

    /// Error: `bump` writes `R`.
    public fun named_fun(a: address) {
        apply_pure(bump, a)
    }

    /// Error: the lambda calls a function that writes `R`.
    public fun lambda_call(a: address) {
        apply_pure(|x| bump(x), a)
    }

    /// Error: the lambda writes `R`.
    public fun lambda_write(a: address) {
        apply_pure(|x| { R[x].value = R[x].value + 1; }, a)
    }

    /// Error: the helper's specification declares the write.
    public fun helper_with_spec(a: address) {
        apply_pure(bump_with_spec, a)
    }

    /// Error: `read_s` reads `S`.
    public fun undeclared_read(a: address): u64 {
        apply_reads_r(read_s, a)
    }

    /// No error: the write is declared.
    public fun declared_write(a: address) {
        apply_writes_r(bump, a)
    }

    /// No error: the read is declared.
    public fun declared_read(a: address): u64 {
        apply_reads_r(|x| R[x].value, a)
    }
}

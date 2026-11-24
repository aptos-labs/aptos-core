// A collection forced inside natives after their first allocation (`split_bytes`,
// `type_info::type_of`, a failing `reflect::resolve`, ed25519 key generation) and inside
// `vector::move_range` as it grows `to`. Each `_fill` step shows the heap is sized so that
// the collection falls inside the native. The `_churned` steps run two more collections
// before the results are read, so a stale pointer would read overwritten memory. The
// `_verified` steps also sign and verify with the keys.

// RUN: publish
module 0x1::test_natives {
    native public fun split_bytes(bytes: vector<u8>, chunk: u64): vector<vector<u8>>;
}
module 0x1::type_info {
    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }

    public native fun type_of<T>(): TypeInfo;

    // Moves the fields out, without copying.
    public fun parts(info: TypeInfo): (address, vector<u8>, vector<u8>) {
        let TypeInfo { account_address, module_name, struct_name } = info;
        (account_address, module_name, struct_name)
    }
}
module 0x1::ed25519 {
    native fun generate_keys_internal(): (vector<u8>, vector<u8>);
    native fun sign_internal(sk: vector<u8>, msg: vector<u8>): vector<u8>;
    native fun signature_verify_strict_internal(
        signature: vector<u8>,
        public_key: vector<u8>,
        message: vector<u8>,
    ): bool;

    public fun keys(): (vector<u8>, vector<u8>) {
        generate_keys_internal()
    }

    public fun verify(sk: vector<u8>, pk: vector<u8>): bool {
        let msg = b"m";
        signature_verify_strict_internal(sign_internal(sk, msg), pk, msg)
    }
}
module 0x42::main {
    use std::features;
    use std::reflect;
    use std::string::utf8;
    use 0x1::test_natives::split_bytes;

    struct Pair<T, U> has drop { a: T, b: U }

    struct Bar<T> has drop { x: T }

    enum Unit has drop { Only }

    const DATA: vector<u8> = b"abcdefghij";
    const KEY_SIZED: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000000";

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    // Moves each chunk out, so the results read every byte without copying.
    fun chunks_of(
        chunks: vector<vector<u8>>,
    ): (u64, vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        let count = chunks.length();
        let fourth = chunks.pop_back();
        let third = chunks.pop_back();
        let second = chunks.pop_back();
        let first = chunks.pop_back();
        (count, first, second, third, fourth)
    }

    // Allocates the four chunks, then the outer vector.
    public fun split(rounds: u64): (u64, vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        churn(rounds);
        chunks_of(split_bytes(DATA, 3))
    }

    public fun split_churned(
        rounds: u64,
        more: u64,
    ): (u64, vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        churn(rounds);
        let chunks = split_bytes(DATA, 3);
        churn(more);
        chunks_of(chunks)
    }

    public fun split_fill(rounds: u64): (u64, u64) {
        churn(rounds);
        let data = DATA;
        let first = vector[1u8, 2, 3];
        (data.length(), first.length())
    }

    // Allocates the module name, then the struct name.
    public fun type_of_pair(rounds: u64): (address, vector<u8>, vector<u8>) {
        churn(rounds);
        let info = 0x1::type_info::type_of<Pair<u64, Bar<u8>>>();
        0x1::type_info::parts(info)
    }

    public fun type_of_churned(rounds: u64, more: u64): (address, vector<u8>, vector<u8>) {
        churn(rounds);
        let info = 0x1::type_info::type_of<Pair<u64, Bar<u8>>>();
        churn(more);
        0x1::type_info::parts(info)
    }

    public fun type_of_fill(rounds: u64): u64 {
        churn(rounds);
        let first = b"main";
        first.length()
    }

    // Fails to resolve: allocates the `ReflectionError`, then the `Result`.
    public fun reflect_err(rounds: u64): u64 {
        churn(rounds);
        let module_name = utf8(b"nomod");
        let func_name = utf8(b"f");
        reflect::resolve<|u64|u64>(@0x42, &module_name, &func_name).unwrap_err().error_code()
    }

    public fun reflect_err_churned(rounds: u64, more: u64): u64 {
        churn(rounds);
        let module_name = utf8(b"nomod");
        let func_name = utf8(b"f");
        let result = reflect::resolve<|u64|u64>(@0x42, &module_name, &func_name);
        churn(more);
        result.unwrap_err().error_code()
    }

    public fun reflect_err_fill(rounds: u64): bool {
        churn(rounds);
        let module_name = utf8(b"nomod");
        let func_name = utf8(b"f");
        let enabled = features::is_function_reflection_enabled();
        let _first = Unit::Only;
        enabled && module_name.length() + func_name.length() > 0
    }

    // Allocates less than one churn round, so the preamble can end where this
    // still fits and growing `to` does not.
    fun pad(): u64 {
        let junk = vector[1u64];
        junk.length()
    }

    // Moves each element out of the one-element `from` and the three-element
    // `to`, so the results read every element without copying.
    fun moved_of(
        from: vector<vector<u64>>,
        to: vector<vector<u64>>,
    ): (u64, vector<u64>, u64, vector<u64>, vector<u64>, vector<u64>) {
        let from_len = from.length();
        let to_len = to.length();
        let from_first = from.pop_back();
        let to_third = to.pop_back();
        let to_second = to.pop_back();
        let to_first = to.pop_back();
        (from_len, from_first, to_len, to_first, to_second, to_third)
    }

    // Inserts into a full `to`, so the native grows it.
    public fun move_range_grow(
        rounds: u64,
    ): (u64, vector<u64>, u64, vector<u64>, vector<u64>, vector<u64>) {
        let from = vector[vector[1u64], vector[2u64, 2], vector[3u64, 3, 3]];
        let to = vector[vector[9u64]];
        churn(rounds);
        let _ = pad();
        std::vector::move_range(&mut from, 0, 2, &mut to, 1);
        moved_of(from, to)
    }

    public fun move_range_churned(
        rounds: u64,
        more: u64,
    ): (u64, vector<u64>, u64, vector<u64>, vector<u64>, vector<u64>) {
        let from = vector[vector[1u64], vector[2u64, 2], vector[3u64, 3, 3]];
        let to = vector[vector[9u64]];
        churn(rounds);
        let _ = pad();
        std::vector::move_range(&mut from, 0, 2, &mut to, 1);
        churn(more);
        moved_of(from, to)
    }

    public fun move_range_fill(rounds: u64): (u64, u64) {
        let from = vector[vector[1u64], vector[2u64, 2], vector[3u64, 3, 3]];
        let to = vector[vector[9u64]];
        churn(rounds);
        let _ = pad();
        (from.length(), to.length())
    }

    // Allocates the secret key, then the public key.
    public fun keygen(rounds: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::ed25519::keys();
        (sk.length(), pk.length(), sk != pk)
    }

    public fun keygen_verified(rounds: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::ed25519::keys();
        (sk.length(), pk.length(), 0x1::ed25519::verify(sk, pk))
    }

    public fun keygen_churned(rounds: u64, more: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::ed25519::keys();
        churn(more);
        (sk.length(), pk.length(), sk != pk)
    }

    public fun keygen_churned_verified(rounds: u64, more: u64): (u64, u64, bool) {
        churn(rounds);
        let (sk, pk) = 0x1::ed25519::keys();
        churn(more);
        (sk.length(), pk.length(), 0x1::ed25519::verify(sk, pk))
    }

    public fun keygen_fill(rounds: u64): u64 {
        churn(rounds);
        let first = KEY_SIZED;
        first.length()
    }
}

// RUN: execute 0x42::main::split_fill --args 2720 --heap-size 32768
// CHECK: results: 10, 3
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::split --args 2720 --heap-size 32768
// CHECK: results: 4, 0x616263, 0x646566, 0x676869, 0x6a
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::split_churned --args 2720, 6800 --heap-size 32768
// CHECK: results: 4, 0x616263, 0x646566, 0x676869, 0x6a
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::split_fill --args 2724 --heap-size 32768
// CHECK: results: 10, 3
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::split --args 2724 --heap-size 32768
// CHECK: results: 4, 0x616263, 0x646566, 0x676869, 0x6a
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::split_churned --args 2724, 6800 --heap-size 32768
// CHECK: results: 4, 0x616263, 0x646566, 0x676869, 0x6a
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::type_of_fill --args 2728 --heap-size 32768
// CHECK: results: 4
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::type_of_pair --args 2728 --heap-size 32768
// CHECK: results: 0x42, 0x6d61696e, 0x506169723c7536342c20307834323a3a6d61696e3a3a4261723c75383e3e
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::type_of_churned --args 2728, 6800 --heap-size 32768
// CHECK: results: 0x42, 0x6d61696e, 0x506169723c7536342c20307834323a3a6d61696e3a3a4261723c75383e3e
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::reflect_err_fill --args 2724 --heap-size 32768
// CHECK: results: true
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::reflect_err --args 2724 --heap-size 32768
// CHECK: results: 1
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::reflect_err_churned --args 2724, 6800 --heap-size 32768
// CHECK: results: 1
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::move_range_fill --args 2712 --heap-size 32768
// CHECK: results: 3, 1
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::move_range_grow --args 2712 --heap-size 32768
// CHECK: results: 1, [3, 3, 3], 3, [9], [1], [2, 2]
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::move_range_churned --args 2712, 6800 --heap-size 32768
// CHECK: results: 1, [3, 3, 3], 3, [9], [1], [2, 2]
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::keygen_fill --args 2724 --heap-size 32768
// CHECK: results: 32
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::keygen --args 2724 --heap-size 32768
// CHECK: results: 32, 32, true
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::keygen_verified --args 2724 --heap-size 32768
// CHECK: results: 32, 32, true
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::keygen_churned --args 2724, 6800 --heap-size 32768
// CHECK: results: 32, 32, true
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::keygen_churned_verified --args 2724, 6800 --heap-size 32768
// CHECK: results: 32, 32, true
// CHECK-GC-COUNT: 3

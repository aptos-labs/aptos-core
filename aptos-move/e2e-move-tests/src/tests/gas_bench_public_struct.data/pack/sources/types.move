/// Types module: baseline benchmarks for struct and enum operations
/// from the DEFINING module (direct bytecode instructions, no accessor calls).
module 0xcafe::gas_bench_types {
    use std::signer;

    /// Struct declared `public` so that external modules can access fields directly.
    public struct Config has copy, drop {
        a: u64,
        b: u64,
        c: u64,
        d: u64,
    }

    /// Enum declared `public` so that external modules can construct and test variants directly.
    public enum Shape has copy, drop {
        Circle { radius: u64 },
        Square { side: u64 },
    }

    struct DirectResult   has key { value: u64 }
    struct UnpackResult   has key { value: u64 }
    struct PackResult     has key { value: u64 }
    struct EnumPackResult has key { value: u64 }
    struct EnumTestResult has key { value: u64 }

    public fun new_config(a: u64, b: u64, c: u64, d: u64): Config {
        Config { a, b, c, d }
    }

    // ── struct benchmarks ────────────────────────────────────────────────────

    /// Baseline: direct field read from the defining module.
    /// Compiles to ImmBorrowField — no function-call overhead.
    public entry fun bench_direct(account: &signer, n: u64) {
        let config = new_config(10, 20, 30, 40);
        let sum = 0u64;
        let i = 0u64;
        while (i < n) {
            sum = sum + config.a + config.b + config.c + config.d;
            i = i + 1;
        };
        move_to(account, DirectResult { value: sum });
    }

    /// Baseline: struct unpack from the defining module.
    /// Compiles to Unpack — no function-call overhead.
    public entry fun bench_unpack(account: &signer, n: u64) {
        let config = new_config(10, 20, 30, 40);
        let sum = 0u64;
        let i = 0u64;
        while (i < n) {
            let Config { a, b, c, d } = config;
            sum = sum + a + b + c + d;
            i = i + 1;
        };
        move_to(account, UnpackResult { value: sum });
    }

    /// Baseline: struct pack from the defining module.
    /// Compiles to Pack — no function-call overhead.
    public entry fun bench_pack(account: &signer, n: u64) {
        let i = 0u64;
        while (i < n) {
            let _config = Config { a: i, b: i + 1, c: i + 2, d: i + 3 };
            i = i + 1;
        };
        move_to(account, PackResult { value: n });
    }

    // ── enum benchmarks ──────────────────────────────────────────────────────

    /// Baseline: enum variant construction from the defining module.
    /// Compiles to PackVariant — no function-call overhead.
    public entry fun bench_enum_pack(account: &signer, n: u64) {
        let i = 0u64;
        while (i < n) {
            let _s = Shape::Circle { radius: i };
            i = i + 1;
        };
        move_to(account, EnumPackResult { value: n });
    }

    /// Baseline: enum variant test (match) from the defining module.
    /// Compiles to TestVariant — no function-call overhead.
    public entry fun bench_enum_test(account: &signer, n: u64) {
        let shape = Shape::Circle { radius: 42 };
        let sum = 0u64;
        let i = 0u64;
        while (i < n) {
            let v = match (&shape) {
                Shape::Circle { .. } => 1u64,
                Shape::Square { .. } => 0u64,
            };
            sum = sum + v;
            i = i + 1;
        };
        move_to(account, EnumTestResult { value: sum });
    }
}

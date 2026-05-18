/// Consumer module: cross-module benchmarks for struct and enum operations.
/// Each operation is only possible because Config/Shape are declared `public`.
/// The compiler converts each cross-module field/variant operation into an
/// auto-generated accessor call, incurring function-call overhead vs. the
/// same-module baseline in gas_bench_types.
module 0xcafe::gas_bench_consumer {
    use std::signer;
    use 0xcafe::gas_bench_types::{Self, Config, Shape};

    struct DirectResult   has key { value: u64 }
    struct UnpackResult   has key { value: u64 }
    struct PackResult     has key { value: u64 }
    struct EnumPackResult has key { value: u64 }
    struct EnumTestResult has key { value: u64 }

    // ── struct benchmarks ────────────────────────────────────────────────────

    /// Cross-module direct field read — each `config.x` becomes an accessor call.
    public entry fun bench_direct(account: &signer, n: u64) {
        let config: Config = gas_bench_types::new_config(10, 20, 30, 40);
        let sum = 0u64;
        let i = 0u64;
        while (i < n) {
            sum = sum + config.a + config.b + config.c + config.d;
            i = i + 1;
        };
        move_to(account, DirectResult { value: sum });
    }

    /// Cross-module struct unpack — becomes an auto-generated unpack accessor call.
    public entry fun bench_unpack(account: &signer, n: u64) {
        let config: Config = gas_bench_types::new_config(10, 20, 30, 40);
        let sum = 0u64;
        let i = 0u64;
        while (i < n) {
            let Config { a, b, c, d } = config;
            sum = sum + a + b + c + d;
            i = i + 1;
        };
        move_to(account, UnpackResult { value: sum });
    }

    /// Cross-module struct pack — becomes an auto-generated pack accessor call.
    public entry fun bench_pack(account: &signer, n: u64) {
        let i = 0u64;
        while (i < n) {
            let _config = Config { a: i, b: i + 1, c: i + 2, d: i + 3 };
            i = i + 1;
        };
        move_to(account, PackResult { value: n });
    }

    // ── enum benchmarks ──────────────────────────────────────────────────────

    /// Cross-module enum variant construction — becomes an auto-generated pack variant call.
    public entry fun bench_enum_pack(account: &signer, n: u64) {
        let i = 0u64;
        while (i < n) {
            let _s = Shape::Circle { radius: i };
            i = i + 1;
        };
        move_to(account, EnumPackResult { value: n });
    }

    /// Cross-module enum variant test — becomes an auto-generated test variant call.
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

// SCRATCH-BRANCH EXPERIMENT, DO NOT MERGE. Shifts all downstream code by
// LAYOUT_PAD bytes (via a padding blob linked first) and times the gated
// programs, so different pads = same source, different code placement.
// LAYOUT_ONLY_MONO=1 skips the native controls. LAYOUT_WORK_SCALE=<f> scales the
// work of every mono program except fib by roughly that factor (synthetic regression).

use mono_move_testsuite::{programs, with_loaded_mono_function, SourceKind};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

const fn parse_pad(bytes: &[u8]) -> usize {
    let mut value = 0;
    let mut index = 0;
    while index < bytes.len() {
        value = value * 10 + (bytes[index] - b'0') as usize;
        index += 1;
    }
    value
}

const PAD: usize = parse_pad(env!("LAYOUT_PAD").as_bytes());

#[cfg(target_os = "macos")]
core::arch::global_asm!(
    ".text",
    ".globl _layout_exp_pad",
    ".p2align 4",
    "_layout_exp_pad:",
    ".space {pad}",
    "ret",
    pad = const PAD,
);

#[cfg(not(target_os = "macos"))]
core::arch::global_asm!(
    ".text",
    ".globl layout_exp_pad",
    ".p2align 4",
    "layout_exp_pad:",
    ".space {pad}",
    "ret",
    pad = const PAD,
);

unsafe extern "C" {
    fn layout_exp_pad();
}

const ROUNDS: usize = 15;
const ROUND_TARGET: Duration = Duration::from_millis(50);
const WARMUP: Duration = Duration::from_millis(200);

fn measure(name: &str, mut workload: impl FnMut() -> u64) {
    let start = Instant::now();
    black_box(workload());
    let single = start.elapsed().max(Duration::from_nanos(1));
    let iters = (ROUND_TARGET.as_nanos() / single.as_nanos()).max(1) as u64;

    let warm_start = Instant::now();
    while warm_start.elapsed() < WARMUP {
        black_box(workload());
    }

    let mut per_call_ns = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        let round_start = Instant::now();
        for _ in 0..iters {
            black_box(workload());
        }
        per_call_ns.push(round_start.elapsed().as_nanos() as f64 / iters as f64);
    }
    per_call_ns.sort_by(|lhs, rhs| lhs.partial_cmp(rhs).unwrap());
    println!(
        "{{\"id\": \"{}\", \"pad\": {}, \"min_ns\": {:.1}, \"median_ns\": {:.1}}}",
        name,
        PAD,
        per_call_ns[0],
        per_call_ns[ROUNDS / 2]
    );
}

fn measure_mono(name: &str, source: &str, module: &str, function: &str, args: &[u64]) {
    let addr = AccountAddress::from_hex_literal("0x1").unwrap();
    with_loaded_mono_function(
        source,
        SourceKind::Move,
        addr,
        IdentStr::new(module).unwrap(),
        IdentStr::new(function).unwrap(),
        |runner| measure(name, || runner.call_words(args).unwrap()),
    )
    .unwrap();
}

fn main() {
    black_box(layout_exp_pad as unsafe extern "C" fn());

    if std::env::var_os("LAYOUT_ONLY_MONO").is_none() {
        run_natives();
    }
    run_monos();
}

fn run_natives() {
    measure("fib/native", || programs::fib::native_fib(25));
    measure("nested_loop/native", || {
        programs::nested_loop::native_nested_loop(1000)
    });
    measure("merge_sort/native", || {
        programs::merge_sort::native_sort_checksum(1000, 42)
    });
    measure("bst/native", || {
        programs::bst::native_run_ops_checksum(5000, 2500, 42)
    });
    measure("match_sum/native", || {
        programs::match_sum::native_match_sum(1_000_000)
    });
    measure("int_arith_loop/native_u64", || {
        programs::int_arith_loop::native_u64_loop(1000)
    });
    measure("int_arith_loop/native_i64", || {
        programs::int_arith_loop::native_i64_loop(1000) as u64
    });
}

fn work_scale() -> f64 {
    std::env::var("LAYOUT_WORK_SCALE")
        .map(|text| text.parse().expect("LAYOUT_WORK_SCALE must be a float"))
        .unwrap_or(1.0)
}

fn scaled(value: u64, factor: f64) -> u64 {
    (value as f64 * factor).round() as u64
}

fn run_monos() {
    let scale = work_scale();
    measure_mono("fib/mono", programs::fib::SOURCE, "fib", "fib", &[25]);
    measure_mono(
        "nested_loop/mono",
        programs::nested_loop::SOURCE,
        "nested_loop",
        "nested_loop",
        &[scaled(1000, scale.sqrt())],
    );
    measure_mono(
        "merge_sort/mono",
        programs::merge_sort::SOURCE,
        "merge_sort",
        "sort_checksum",
        &[scaled(1000, scale), 42],
    );
    measure_mono(
        "bst/mono",
        programs::bst::SOURCE,
        "bst",
        "run_ops_checksum",
        &[scaled(5000, scale), 2500, 42],
    );
    measure_mono(
        "match_sum/mono",
        programs::match_sum::SOURCE,
        "match_sum",
        "match_sum",
        &[scaled(1_000_000, scale)],
    );
    measure_mono(
        "int_arith_loop/mono_u64",
        programs::int_arith_loop::SOURCE,
        "int_arith_loop",
        "u64_loop",
        &[scaled(1000, scale)],
    );
    measure_mono(
        "int_arith_loop/mono_i64",
        programs::int_arith_loop::SOURCE,
        "int_arith_loop",
        "i64_loop",
        &[scaled(1000, scale)],
    );
}

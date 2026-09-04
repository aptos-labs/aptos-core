// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ATT017 regression. A generic closure used to cost a flat 40 abstract units
//! (`misc.abs_val.closure`) no matter how large its type arguments were, even
//! though those type arguments are materialized as `TypeTag`s and re-serialized
//! on every BCS round trip. The abstract memory quota is 10,000,000 units, so a
//! single transaction could hold 250,000 closures whose tags were hundreds of
//! kilobytes each.
//!
//! `TimedFeatureFlag::MeterClosureTypeArguments` adds the type arguments'
//! pseudo-gas cost to the closure's abstract size. The tests below measure the
//! charge with the flag off and on and check it against the pricing formula.
//!
//! Note this moves the abstract memory charge, not gas used: the quota is a
//! separate meter, so `gas_used` is the same either way.
//!
//! Run with:
//!   RUST_MIN_STACK=1073741824 cargo test -p e2e-move-tests \
//!       --test closure_ty_tag_memory -- --nocapture --test-threads=1

use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::TimedFeatureFlag,
    transaction::{AuxiliaryInfo, TransactionStatus},
};
use e2e_move_tests::MoveHarness;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Module generation
// ---------------------------------------------------------------------------

const ADDR: &str = "0xbeef";

/// Identifiers are capped at 255 bytes. The per-struct-node pseudo gas cost is
/// `type_base_cost + address_len + module_len + struct_len` = `100 + 32 + 250 +
/// 240` = 622, and `type_max_cost` is 5000, so a chain of 8 struct nodes costs
/// 4976 and just fits.
const MODULE_LEN: usize = 250;
const STRUCT_LEN: usize = 240;

/// The type tag pricing the VM applies, from `aptos_prod_vm_config`.
const TYPE_BASE_COST: u64 = 100;
const TYPE_BYTE_COST: u64 = 1;

/// `misc.abs_val.closure`, the flat part of a closure's abstract size.
const CLOSURE_BASE: u64 = 40;

struct Shape {
    label: &'static str,
    num_ty_args: usize,
    /// Number of struct nodes in each type argument. Zero means `bool`.
    nest: usize,
}

impl Shape {
    /// Recomputes the VM's own pricing: a `type_base_cost` per type tag node,
    /// plus a `type_byte_cost` per byte of address and identifiers in each
    /// struct node.
    fn expected_charge(&self, metered: bool) -> u64 {
        if !metered {
            return CLOSURE_BASE;
        }
        let per_ty_arg = if self.nest == 0 {
            // `bool` is a single node with no identifiers.
            TYPE_BASE_COST
        } else {
            let per_struct_node = TYPE_BASE_COST
                + TYPE_BYTE_COST * (AccountAddress::LENGTH + MODULE_LEN + STRUCT_LEN) as u64;
            self.nest as u64 * per_struct_node
        };
        CLOSURE_BASE + self.num_ty_args as u64 * per_ty_arg
    }
}

fn pad(prefix: &str, len: usize) -> String {
    let mut s = prefix.to_string();
    assert!(s.len() <= len);
    while s.len() < len {
        s.push('q');
    }
    s
}

/// Builds the nested type expression, e.g. `W<W<W<B>>>` for `nest == 4`.
fn nested_ty(wrapper: &str, base: &str, nest: usize) -> String {
    if nest == 0 {
        return "bool".to_string();
    }
    let mut ty = base.to_string();
    for _ in 0..nest - 1 {
        ty = format!("{}<{}>", wrapper, ty);
    }
    ty
}

fn module_source(module: &str, shape: &Shape) -> String {
    let wrapper = pad("W", STRUCT_LEN);
    let base = pad("B", STRUCT_LEN);
    let ty = nested_ty(&wrapper, &base, shape.nest);

    let (ty_params, ty_args) = if shape.num_ty_args == 0 {
        (String::new(), String::new())
    } else {
        let params = (0..shape.num_ty_args)
            .map(|i| format!("T{}", i))
            .collect::<Vec<_>>()
            .join(", ");
        let args = vec![ty.as_str(); shape.num_ty_args].join(", ");
        (format!("<{}>", params), format!("<{}>", args))
    };

    // The publish-time complexity budget is `2048 + 20 * module_blob_len`
    // (`AptosVM::validate_publish_request`), while the complexity of the
    // instantiation signature is `num_ty_args * nest * (8 + module_len +
    // struct_len)`. Padding the blob with a constant buys budget at 20x, so the
    // check does not actually bound how large the instantiation can be.
    let needed = shape.num_ty_args * shape.nest.max(1) * (8 + MODULE_LEN + STRUCT_LEN);
    let pad_bytes = (needed / 4).max(1024);
    let pad_hex = "ab".repeat(pad_bytes);

    format!(
        r#"
module {ADDR}::{module} {{
    use std::vector;

    const PAD: vector<u8> = x"{pad_hex}";

    public fun pad(): vector<u8> {{ PAD }}

    struct {base} has copy, drop, store {{ dummy: bool }}
    struct {wrapper}<phantom T> has copy, drop, store {{ dummy: bool }}

    #[persistent]
    public fun target{ty_params}() {{ }}

    // Behind a call boundary so the closure cannot be hoisted out of the loop.
    public fun make(): || has copy+drop+store {{
        target{ty_args}
    }}

    public entry fun run(n: u64) {{
        let v = vector::empty();
        let i = 0;
        while (i < n) {{
            vector::push_back(&mut v, make());
            i = i + 1;
        }};
        assert!(vector::length(&v) == n, 1);
    }}
}}
"#
    )
}

fn framework_dir_path(s: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("framework")
        .join(s)
}

fn publish(h: &mut MoveHarness, account: &Account, package: &str, source: &str) {
    let mut builder = PackageBuilder::new(package);
    builder.add_source("m.move", source);
    builder.add_local_dep(
        "MoveStdlib",
        &framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    let status = h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    );
    assert!(
        matches!(status, TransactionStatus::Keep(ref s) if s.is_success()),
        "publish failed: {:?}",
        status
    );
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

/// Both must stay under the abstract memory quota for the metered worst case.
const SMALL: u64 = 10;
const LARGE: u64 = 30;

fn harness(metered: bool) -> MoveHarness {
    let mut h = MoveHarness::new_testnet();
    h.max_gas_per_txn = 2_000_000;
    h.set_timed_feature(TimedFeatureFlag::MeterClosureTypeArguments, metered);
    h
}

/// Peak abstract memory usage of `run(n)`, which is what `txn.memory_quota`
/// bounds. Only the profiler reports it.
fn peak_memory(h: &mut MoveHarness, account: &Account, module: &str, n: u64) -> u64 {
    let txn = h.create_entry_function(
        account,
        str::parse(&format!("{}::{}::run", ADDR, module)).unwrap(),
        vec![],
        vec![bcs::to_bytes(&n).unwrap()],
    );
    let (status, gas_log, ..) =
        h.evaluate_gas_with_profiler_and_status_signed(txn, &AuxiliaryInfo::default());
    assert!(
        matches!(status, TransactionStatus::Keep(ref s) if s.is_success()),
        "run({}) failed: {:?}",
        n,
        status
    );
    u64::from(gas_log.peak_memory_usage)
}

/// Abstract memory charged per closure. Taking a slope between two closure
/// counts cancels the transaction's fixed overhead.
fn charge_per_closure(h: &mut MoveHarness, account: &Account, module: &str) -> u64 {
    let small = peak_memory(h, account, module, SMALL);
    let large = peak_memory(h, account, module, LARGE);
    (large - small) / (LARGE - SMALL)
}

fn charges_per_closure(shapes: &[Shape], metered: bool) -> Vec<u64> {
    let mut h = harness(metered);
    let account = h.new_account_at(AccountAddress::from_hex_literal(ADDR).unwrap());
    shapes
        .iter()
        .enumerate()
        .map(|(i, shape)| {
            let module = pad(&format!("m{}", i), MODULE_LEN);
            publish(
                &mut h,
                &account,
                &format!("Package{}", i),
                &module_source(&module, shape),
            );
            charge_per_closure(&mut h, &account, &module)
        })
        .collect::<Vec<u64>>()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The charge for a closure with large type arguments before and after the flag.
/// Four orders of magnitude, which is what drops the number of closures that fit
/// under the quota from 250,000 to 62.
#[test]
fn closure_type_arguments_are_metered() {
    let shape = Shape {
        label: "32 x depth-8",
        num_ty_args: 32,
        nest: 8,
    };
    let shapes = std::slice::from_ref(&shape);

    let unmetered = charges_per_closure(shapes, false)[0];
    let metered = charges_per_closure(shapes, true)[0];

    let quota = 10_000_000u64;
    println!("\n=== {} ===", shape.label);
    println!(
        "flag off: {:>9} units/closure, {:>9} closures under quota",
        unmetered,
        quota / unmetered
    );
    println!(
        "flag on:  {:>9} units/closure, {:>9} closures under quota",
        metered,
        quota / metered
    );

    assert_eq!(unmetered, shape.expected_charge(false));
    assert_eq!(metered, shape.expected_charge(true));
    assert!(metered > 1000 * unmetered);
}

/// The charge must scale with the size of the type arguments, in both the number
/// of arguments and the size of each. Charging the same flat base for all of
/// them is the bug.
#[test]
fn closure_charge_scales_with_type_argument_size() {
    let shapes = [
        Shape {
            label: "0 type args",
            num_ty_args: 0,
            nest: 0,
        },
        Shape {
            label: "8 x bool",
            num_ty_args: 8,
            nest: 0,
        },
        Shape {
            label: "8 x depth-2",
            num_ty_args: 8,
            nest: 2,
        },
        Shape {
            label: "8 x depth-8",
            num_ty_args: 8,
            nest: 8,
        },
        Shape {
            label: "32 x depth-8",
            num_ty_args: 32,
            nest: 8,
        },
    ];

    let unmetered = charges_per_closure(&shapes, false);
    let metered = charges_per_closure(&shapes, true);

    println!("\n{:>16}  {:>12}  {:>12}", "shape", "flag off", "flag on");
    for (i, shape) in shapes.iter().enumerate() {
        println!(
            "{:>16}  {:>12}  {:>12}",
            shape.label, unmetered[i], metered[i]
        );
    }

    for (i, shape) in shapes.iter().enumerate() {
        assert_eq!(
            unmetered[i],
            shape.expected_charge(false),
            "unmetered charge for {}",
            shape.label
        );
        assert_eq!(
            metered[i],
            shape.expected_charge(true),
            "metered charge for {}",
            shape.label
        );
    }

    // The shapes are listed in increasing order of tag size.
    assert!(unmetered.iter().all(|c| *c == CLOSURE_BASE));
    assert!(metered.windows(2).all(|w| w[0] < w[1]));

    // A closure with no type arguments is unaffected.
    assert_eq!(metered[0], unmetered[0]);
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Publishes a module that is legal under the production verifier and the
//! publish-time complexity check, then fetches it back over the REST API and
//! measures what the ABI expansion costs the node.
//!
//! Two gaps meet here:
//!
//!   1. `check_module_complexity` charges struct field types via `num_nodes()`
//!      -- one unit per node, with no cost for the declaring module name, the
//!      struct name, or the address (`check_complexity.rs:243`). The signature
//!      table charges 8 + both identifier lengths for the same node.
//!   2. `MoveStructTag` carries all three at every node, and `new_move_type`
//!      (`api/types/src/bytecode.rs:100`) recurses with no budget, no depth
//!      limit and no node counter.
//!
//! `MAX_RECURSIVE_TYPES_ALLOWED = 8` does not apply: it is only reachable via
//! `VerifyInput`, which validates incoming request payloads, not ABI output.

use aptos_api_test_context::{current_function_name, new_test_context_inner, TestContext};
use aptos_api_types::MoveModuleBytecode;
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::{internal_indexer_db_config::InternalIndexerDBConfig, NodeConfig};
use aptos_framework::natives::code::{ModuleMetadata, PackageMetadata, UpgradePolicy};
use aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION;
use aptos_sdk::types::LocalAccount;
use aptos_types::{
    on_chain_config::{Features, TimedFeaturesBuilder},
    transaction::TransactionPayload,
};
use move_binary_format::{
    file_format::{
        AddressIdentifierIndex, FieldDefinition, IdentifierIndex, ModuleHandle, ModuleHandleIndex,
        SignatureToken, StructDefinition, StructFieldInformation, StructHandle, StructHandleIndex,
        StructTypeParameter, TypeSignature,
    },
    file_format_common::VERSION_MAX,
    CompiledModule,
};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering},
};

/// `IDENTIFIER_SIZE_MAX`, the longest identifier the deserializer accepts.
const MAX_IDENT: usize = 255;
/// `max_fields_in_struct` in the production verifier config.
const FIELDS_PER_STRUCT: usize = 64;
/// Type arguments to `A`. `verify_type_node` weighs a struct node at 4 against
/// `max_type_nodes` (128), so 1 + 31 struct nodes lands exactly on the limit.
/// `max_generic_instantiation_length` (32) also permits 31.
const TYPE_ARGS: usize = 31;
/// Container structs per module. Keeps the blob under
/// `max_transaction_size_in_bytes` (64 KiB), so no chunked publish is needed.
const CONTAINERS: usize = 14;

fn ident(s: String) -> Identifier {
    Identifier::new(s).unwrap()
}

/// A maximum-length identifier, distinguished by `tag`.
fn long_ident(tag: &str) -> Identifier {
    let mut s = tag.to_string();
    s.push_str(&"z".repeat(MAX_IDENT - s.len()));
    ident(s)
}

/// `A<B, B, ..., B>`: the most tag-rendering nodes one field type can hold.
fn field_type() -> SignatureToken {
    SignatureToken::StructInstantiation(StructHandleIndex(0), vec![
        SignatureToken::Struct(
            StructHandleIndex(1)
        );
        TYPE_ARGS
    ])
}

/// Builds a module with `CONTAINERS` structs of `FIELDS_PER_STRUCT` fields
/// each, where every field has the type above.
///
/// The module, `A` and `B` carry maximum-length names. Those are stored once in
/// the identifier table but re-emitted at every node of the ABI.
fn build_module(address: AccountAddress, module_name: Identifier) -> CompiledModule {
    // 0: module name, 1: `A`, 2: `B`.
    let mut identifiers = vec![module_name, long_ident("a"), long_ident("b")];

    // Field names, shared by every container struct.
    let first_field_name = identifiers.len() as u16;
    for i in 0..FIELDS_PER_STRUCT {
        identifiers.push(ident(format!("f{}", i)));
    }

    // Then one name per container struct.
    let first_container_name = identifiers.len() as u16;
    for i in 0..CONTAINERS {
        identifiers.push(ident(format!("C{}", i)));
    }

    let no_constraint = StructTypeParameter {
        constraints: AbilitySet::EMPTY,
        is_phantom: false,
    };
    let mut struct_handles = vec![
        // `A<T0, .., T30>`, unconstrained so any type argument is accepted.
        StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            abilities: AbilitySet::EMPTY,
            type_parameters: vec![no_constraint; TYPE_ARGS],
        },
        // `B`, the leaf.
        StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(2),
            abilities: AbilitySet::EMPTY,
            type_parameters: vec![],
        },
    ];
    for i in 0..CONTAINERS {
        struct_handles.push(StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(first_container_name + i as u16),
            abilities: AbilitySet::EMPTY,
            type_parameters: vec![],
        });
    }

    // `A` and `B` need definitions; a single `u8` field keeps them acyclic.
    let u8_field = |name| {
        StructFieldInformation::Declared(vec![FieldDefinition {
            name: IdentifierIndex(name),
            signature: TypeSignature(SignatureToken::U8),
        }])
    };
    let mut struct_defs = vec![
        StructDefinition {
            struct_handle: StructHandleIndex(0),
            field_information: u8_field(first_field_name),
        },
        StructDefinition {
            struct_handle: StructHandleIndex(1),
            field_information: u8_field(first_field_name),
        },
    ];
    for i in 0..CONTAINERS {
        let fields = (0..FIELDS_PER_STRUCT)
            .map(|f| FieldDefinition {
                name: IdentifierIndex(first_field_name + f as u16),
                signature: TypeSignature(field_type()),
            })
            .collect();
        struct_defs.push(StructDefinition {
            struct_handle: StructHandleIndex(2 + i as u16),
            field_information: StructFieldInformation::Declared(fields),
        });
    }

    CompiledModule {
        version: VERSION_MAX,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        struct_handles,
        function_handles: vec![],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![],
        identifiers,
        address_identifiers: vec![address],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs,
        function_defs: vec![],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    }
}

fn serialize(module: &CompiledModule) -> Vec<u8> {
    let mut blob = vec![];
    module.serialize(&mut blob).unwrap();
    blob
}

/// A `code::publish_package_txn` payload carrying one hand-built module.
fn publish_payload(module: &CompiledModule, package: String) -> TransactionPayload {
    let metadata = PackageMetadata {
        name: package,
        upgrade_policy: UpgradePolicy::compat(),
        upgrade_number: 0,
        source_digest: String::new(),
        manifest: vec![],
        modules: vec![ModuleMetadata {
            name: module.self_id().name().to_string(),
            source: vec![],
            source_map: vec![],
            extension: None,
        }],
        deps: vec![],
        extension: None,
    };
    aptos_stdlib::code_publish_package_txn(bcs::to_bytes(&metadata).unwrap(), vec![serialize(
        module,
    )])
}

/// Measures the blowup without a node: the module is legal, cheap to publish,
/// and enormous once expanded.
#[test]
fn abi_expansion_ratio() {
    let address = AccountAddress::from_hex_literal(
        "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    )
    .unwrap();
    let module = build_module(address, long_ident("m"));

    // The same verifier config mainnet runs, not `VerifierConfig::default()`.
    let config = aptos_vm_environment::prod_configs::aptos_prod_verifier_config(
        LATEST_GAS_FEATURE_VERSION,
        &Features::default(),
        &TimedFeaturesBuilder::enable_all().build(),
    );
    move_bytecode_verifier::verify_module_with_config(&config, &module)
        .expect("module must pass the bytecode verifier");

    let blob = serialize(&module);
    assert!(
        blob.len() < 64 * 1024,
        "must fit in one ordinary transaction"
    );

    // The budget `AptosVM` grants at publish time (`aptos_vm.rs:1682`).
    let budget = 2048 + blob.len() as u64 * 20;
    let used = move_binary_format::check_complexity::check_module_complexity(&module, budget)
        .expect("module must pass the complexity check");

    let abi = MoveModuleBytecode::new(blob.clone())
        .try_parse_abi()
        .unwrap()
        .abi
        .expect("ABI must be produced");
    let json = serde_json::to_string(&abi).unwrap().len();

    let budget_used = used as f64 / budget as f64 * 100.0;
    println!("published bytes:   {}", blob.len());
    println!("complexity budget: {}", budget);
    println!("complexity used:   {} ({:.2}%)", used, budget_used);
    println!("ABI JSON bytes:    {}", json);
    println!("amplification:     {:.0}x", json as f64 / blob.len() as f64);

    assert!(
        json as f64 / blob.len() as f64 > 200.0,
        "expected >200x amplification"
    );
    // The check is nowhere near binding, so capping its budget would not
    // constrain this shape at all.
    assert!(
        budget_used < 5.0,
        "expected the complexity check to be slack"
    );
}

/// The real thing: publish over `POST /transactions`, read back over
/// `GET /accounts/{addr}/modules`, both through the running API.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abi_expansion_via_rest_api() {
    let mut context = new_context(current_function_name!());
    let mut account = context.create_account().await;

    let module = build_module(account.address(), long_ident("m0"));
    let published = serialize(&module).len();
    context
        .publish_package(&mut account, publish_payload(&module, "bomb0".to_string()))
        .await;

    let body = get_modules(&context, &account, 1).await;
    println!("published bytes: {}", published);
    println!("response bytes:  {}", body);
    println!("amplification:   {:.0}x", body as f64 / published as f64);

    assert!(
        body as f64 / published as f64 > 200.0,
        "expected >200x amplification over the wire, got {:.0}x",
        body as f64 / published as f64
    );
}

/// Same request, enough modules to exhaust the heap.
///
/// Ignored by default. `MODULES` sets how many to publish; production allows a
/// page of `DEFAULT_MAX_ACCOUNT_MODULES_PAGE_SIZE` = 9999. `HEAP_LIMIT_MB`
/// caps the heap, standing in for the node's container limit; the cap is armed
/// only once publishing is done, so it bounds the request, not the setup.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn abi_expansion_rest_api_oom() {
    let modules = env("MODULES", 64);
    let heap_limit_mb = env("HEAP_LIMIT_MB", usize::MAX / (1024 * 1024));

    let mut context = new_context(current_function_name!());
    let mut account = context.create_account().await;

    // Storage fees at 400 octas/byte; top up well past what publishing costs.
    let mut root = context.root_account().await;
    context
        .api_execute_aptos_account_transfer(&mut root, account.address(), 100_000_000_000)
        .await;

    let mut published = 0;
    for i in 0..modules {
        let module = build_module(account.address(), long_ident(&format!("m{}", i)));
        published += serialize(&module).len();
        context
            .publish_package(&mut account, publish_payload(&module, format!("bomb{}", i)))
            .await;
        if i % 16 == 0 {
            println!("published {} modules, heap {} MiB", i, heap_mib());
        }
    }
    println!(
        "published {} modules, {:.2} MB on chain, baseline heap {} MiB, arming {} MiB cap",
        modules,
        published as f64 / 1e6,
        heap_mib(),
        heap_limit_mb
    );

    LIMIT.store(heap_limit_mb.saturating_mul(1024 * 1024), Ordering::Relaxed);

    let body = get_modules(&context, &account, 9999).await;
    println!(
        "response bytes: {:.1} MB, heap {} MiB",
        body as f64 / 1e6,
        heap_mib()
    );
}

fn new_context(test_name: String) -> TestContext {
    let mut node_config = NodeConfig::default();
    node_config.indexer_db_config = InternalIndexerDBConfig::new(true, true, true, 0, true, 10);
    let context = new_test_context_inner(test_name, node_config, false, None, false, false);
    context
        .get_indexer_reader()
        .unwrap()
        .wait_for_internal_indexer(0)
        .unwrap();
    context
}

/// `GET /accounts/{addr}/modules?limit={limit}`, returning the response size.
async fn get_modules(context: &TestContext, account: &LocalAccount, limit: usize) -> usize {
    // `reply` skips the wait that `TestContext::execute` does, and the modules
    // endpoint reads through the internal indexer.
    context.wait_for_internal_indexer_caught_up().await;
    let path = context.prepend_path(&format!(
        "/accounts/{}/modules?limit={}",
        account.address().to_hex_literal(),
        limit
    ));
    let resp = context
        .reply(warp::test::request().method("GET").path(&path))
        .await;
    assert_eq!(resp.status(), 200, "unexpected status");
    resp.body().len()
}

fn env(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn heap_mib() -> usize {
    LIVE.load(Ordering::Relaxed) / (1024 * 1024)
}

static LIVE: AtomicUsize = AtomicUsize::new(0);
/// Live-bytes ceiling. Past it, allocation fails, which is what a node under a
/// cgroup or container memory limit sees. `usize::MAX` means uncapped.
static LIMIT: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Tracks live bytes and refuses to hand out more than `LIMIT`. Returning null
/// makes Rust call `handle_alloc_error`, which aborts the process.
struct Capped;

unsafe impl GlobalAlloc for Capped {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if LIVE.load(Ordering::Relaxed) + layout.size() > LIMIT.load(Ordering::Relaxed) {
            return std::ptr::null_mut();
        }
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if LIVE.load(Ordering::Relaxed) + new_size > LIMIT.load(Ordering::Relaxed) {
            return std::ptr::null_mut();
        }
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            LIVE.fetch_add(new_size, Ordering::Relaxed);
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        p
    }
}

#[global_allocator]
static ALLOC: Capped = Capped;

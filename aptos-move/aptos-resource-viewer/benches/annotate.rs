// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Measures the overhead of real-allocation annotation metering.
//!
//! The `metered-allocations` feature changes exactly one thing on the hot path:
//! whether `aptos_jemalloc::current_live_bytes()` performs two jemalloc `mallctl`
//! reads per `Meter::check()` (metering ON) or returns `0` for free (metering OFF).
//! `Meter::check()` is called recursively at every node of the value tree, so the
//! per-node reader cost is the entire cost of the feature.
//!
//! Rather than compile the bench twice, we pass the two readers explicitly:
//!   - `read_metering_off` — a `|| 0` reader, identical to the no-op
//!     `current_live_bytes()` the node links in a metering-OFF build.
//!   - `aptos_jemalloc::current_live_bytes` — the real jemalloc reader.
//! Both run against the same payloads in one `cargo bench`, so the delta between
//! the two groups is a faithful with/without-checks comparison.
//!
//! jemalloc must be the global allocator for the real reader to report live bytes,
//! so this binary installs it (unix only), mirroring `tests/metering.rs`.

use aptos_config::config::DEFAULT_MAX_RESOURCE_ANNOTATION_BYTES;
use criterion::{black_box, Criterion};
use move_binary_format::{
    file_format::{
        empty_module, FieldDefinition, IdentifierIndex, SignatureToken, StructDefinition,
        StructFieldInformation, StructHandle, StructHandleIndex, TypeSignature, VariantDefinition,
    },
    CompiledModule,
};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{MoveStruct, MoveValue},
};
use move_resource_viewer::{AnnotatedMoveValue, CompiledModuleView, MoveValueAnnotator};
use std::{collections::HashMap, sync::Arc};

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// Per-query annotation byte budget for the bench. A bench-only ceiling chosen so nothing
/// aborts mid-measure — we time full annotation, not early exits. Far above the production
/// budget (`DEFAULT_MAX_RESOURCE_ANNOTATION_BYTES` = 100 MB); the Layer-3b structural-edge
/// workloads deliberately exceed that production budget, and `edge_value_depth_128` alone
/// expands a legal 1 MB resource to ~1.5 GB of annotation heap, so the ceiling clears it.
const BUDGET: usize = 4_000_000_000;

/// The single-resource storage-write cap (`max_bytes_per_write_op` = `1 << 20`): the largest
/// BCS blob a resource can be and still be writable on-chain, hence the largest a real API
/// request ever annotates per resource. Every Layer-3b edge workload sizes itself just under
/// this, so it is a *legal* resource rather than a synthetic over-large blob.
const MAX_RESOURCE_BYTES: usize = 1 << 20;

/// Resolves the modules a struct payload references. An empty map mirrors the old
/// `EmptyModuleView`: primitive vectors resolve without any module lookup. The
/// trait is implemented for `&ModuleMapView` so the annotator borrows the view
/// (the annotator is generic with no `Clone`/`'static` bound) — each benchmark
/// iteration constructs a fresh annotator from `&view` with no cloning.
struct ModuleMapView {
    modules: HashMap<ModuleId, Arc<CompiledModule>>,
}

impl ModuleMapView {
    fn empty() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    fn from_modules(modules: Vec<CompiledModule>) -> Self {
        Self {
            modules: modules
                .into_iter()
                .map(|m| (m.self_id(), Arc::new(m)))
                .collect(),
        }
    }
}

impl CompiledModuleView for &ModuleMapView {
    type Item = Arc<CompiledModule>;

    fn view_compiled_module(&self, id: &ModuleId) -> anyhow::Result<Option<Self::Item>> {
        Ok(self.modules.get(id).cloned())
    }
}

/// Models the reader linked when `metered-allocations` is OFF: a trivial `|| 0`.
fn read_metering_off() -> i64 {
    0
}

// ============================================================
// Layer 1: module builders — assemble the 0x1::bench module
// ============================================================

fn intern(m: &mut CompiledModule, s: &str) -> IdentifierIndex {
    let idx = IdentifierIndex(m.identifiers.len() as u16);
    m.identifiers.push(Identifier::new(s).unwrap());
    idx
}

fn vector(tok: SignatureToken) -> SignatureToken {
    SignatureToken::Vector(Box::new(tok))
}

fn struct_tok(idx: StructHandleIndex) -> SignatureToken {
    SignatureToken::Struct(idx)
}

/// Resolves the handle index of the already-defined struct named `name`. Lets a
/// composite shape reference a shared building block by name, so block order does
/// not have to thread index bindings around.
fn struct_idx(m: &CompiledModule, name: &str) -> StructHandleIndex {
    let ident = Identifier::new(name).unwrap();
    let pos = m
        .struct_handles
        .iter()
        .position(|h| m.identifiers[h.name.0 as usize] == ident)
        .expect("struct must be defined before it is referenced");
    StructHandleIndex(pos as u16)
}

/// Appends a struct named `name` with `fields` to module `m`, returning its handle
/// index. Centralizes handle/def bookkeeping so each shape is a few readable lines.
fn define_struct(
    m: &mut CompiledModule,
    name: &str,
    fields: &[(&str, SignatureToken)],
) -> StructHandleIndex {
    let module = m.self_module_handle_idx;
    let name_idx = intern(m, name);
    let handle_idx = StructHandleIndex(m.struct_handles.len() as u16);
    m.struct_handles.push(StructHandle {
        module,
        name: name_idx,
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    });
    let field_defs = fields
        .iter()
        .map(|(fname, tok)| FieldDefinition {
            name: intern(m, fname),
            signature: TypeSignature(tok.clone()),
        })
        .collect();
    m.struct_defs.push(StructDefinition {
        struct_handle: handle_idx,
        field_information: StructFieldInformation::Declared(field_defs),
    });
    handle_idx
}

/// Appends an enum named `name` to module `m`, emitting `DeclaredVariants`. Each
/// `(variant_name, fields)` pair becomes a `VariantDefinition`. Returns the handle
/// index. The only builder that touches variants.
fn define_enum(
    m: &mut CompiledModule,
    name: &str,
    variants: Vec<(&str, Vec<(&str, SignatureToken)>)>,
) -> StructHandleIndex {
    let module = m.self_module_handle_idx;
    let name_idx = intern(m, name);
    let handle_idx = StructHandleIndex(m.struct_handles.len() as u16);
    m.struct_handles.push(StructHandle {
        module,
        name: name_idx,
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    });
    let variant_defs = variants
        .into_iter()
        .map(|(vname, fields)| {
            let name = intern(m, vname);
            let fields = fields
                .into_iter()
                .map(|(fname, tok)| FieldDefinition {
                    name: intern(m, fname),
                    signature: TypeSignature(tok),
                })
                .collect();
            VariantDefinition { name, fields }
        })
        .collect();
    m.struct_defs.push(StructDefinition {
        struct_handle: handle_idx,
        field_information: StructFieldInformation::DeclaredVariants(variant_defs),
    });
    handle_idx
}

/// `0x1::bench::<name>` with no type args.
fn struct_tag(name: &str) -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("bench").unwrap(),
        name: Identifier::new(name).unwrap(),
        type_args: vec![],
    }))
}

/// Hand-builds module `0x1::bench`, the single self-contained module holding every
/// benchmark shape. Built directly with `file_format` (no node, DB, or compiler).
/// Layout: the existing vector-of-structs shape, then shared building blocks reused
/// across resource shapes, then one composite block per resource. Composites resolve
/// shared blocks via `struct_idx`.
fn bench_module() -> CompiledModule {
    let mut m = empty_module();
    // Re-home the self module from the placeholder `0x0::<SELF>` to `0x1::bench`.
    m.address_identifiers[0] = AccountAddress::ONE;
    m.identifiers[0] = Identifier::new("bench").unwrap();

    use SignatureToken::{Address, Bool, U128, U64, U8};

    // --- vector-of-nested-structs shape (existing `vec_of_structs`) ---
    let inner = define_struct(&mut m, "Inner", &[
        ("a", U64),
        ("b", U64),
        ("c", Address),
        ("d", Bool),
    ]);
    define_struct(&mut m, "Outer", &[
        ("items", vector(struct_tok(inner))),
        ("tag", vector(U8)),
        ("n", U128),
    ]);

    // --- shared building blocks (composed by the resource shapes below) ---
    // `Object<T>` annotates as a one-field struct `{ inner: address }` — a struct
    // node, not a bare address — so the topology must keep it a struct.
    define_struct(&mut m, "Object", &[("inner", Address)]);
    define_struct(&mut m, "String", &[("bytes", vector(U8))]);
    let id = define_struct(&mut m, "ID", &[("creation_num", U64), ("addr", Address)]);
    let guid = define_struct(&mut m, "GUID", &[("id", struct_tok(id))]);
    define_struct(&mut m, "EventHandle", &[
        ("counter", U64),
        ("guid", struct_tok(guid)),
    ]);

    // --- FungibleStore (0x1::fungible_asset) — modern default balance store ---
    let object = struct_idx(&m, "Object");
    define_struct(&mut m, "FungibleStore", &[
        ("metadata", struct_tok(object)),
        ("balance", U64),
        ("frozen", Bool),
    ]);

    // --- CoinStore<AptosCoin> (0x1::coin) — legacy, deep EventHandle nesting ---
    let event_handle = struct_idx(&m, "EventHandle");
    let coin = define_struct(&mut m, "Coin", &[("value", U64)]);
    define_struct(&mut m, "CoinStore", &[
        ("coin", struct_tok(coin)),
        ("frozen", Bool),
        ("deposit_events", struct_tok(event_handle)),
        ("withdraw_events", struct_tok(event_handle)),
    ]);

    // --- Token (0x4::token, v2 NFT) — String/byte-heavy + Object + EventHandle ---
    let object = struct_idx(&m, "Object");
    let string = struct_idx(&m, "String");
    let event_handle = struct_idx(&m, "EventHandle");
    define_struct(&mut m, "Token", &[
        ("collection", struct_tok(object)),
        ("index", U64),
        ("description", struct_tok(string)),
        ("name", struct_tok(string)),
        ("uri", struct_tok(string)),
        ("mutation_events", struct_tok(event_handle)),
    ]);

    // --- PropertyMap (0x4::property_map) — string-keyed SimpleMap, distinct leaf mix ---
    let string = struct_idx(&m, "String");
    let property_value = define_struct(&mut m, "PropertyValue", &[
        ("type", U8),
        ("value", vector(U8)),
    ]);
    let element = define_struct(&mut m, "PropertyElement", &[
        ("key", struct_tok(string)),
        ("value", struct_tok(property_value)),
    ]);
    let simple_map = define_struct(&mut m, "PropertySimpleMap", &[(
        "data",
        vector(struct_tok(element)),
    )]);
    define_struct(&mut m, "PropertyMap", &[("inner", struct_tok(simple_map))]);

    // --- VersionedConfig — a Move enum (V1/V2); the only RuntimeVariant workload ---
    // Models the versioned-resource idiom (e.g. confidential_asset::GlobalConfig).
    let auditor = define_struct(&mut m, "AuditorShape", &[("ek", vector(U8))]);
    define_enum(&mut m, "VersionedConfig", vec![
        ("V1", vec![("flag", Bool), ("inner", struct_tok(auditor))]),
        ("V2", vec![
            ("flag", Bool),
            ("inner", struct_tok(auditor)),
            ("paused", Bool),
        ]),
    ]);

    // --- CoinInfo.supply — Option/OptionalAggregator/Aggregator nesting idiom ---
    let aggregator = define_struct(&mut m, "Aggregator", &[
        ("handle", Address),
        ("key", Address),
        ("limit", U128),
    ]);
    let integer = define_struct(&mut m, "Integer", &[("value", U128), ("limit", U128)]);
    let opt_aggregator = define_struct(&mut m, "OptionAggregator", &[(
        "vec",
        vector(struct_tok(aggregator)),
    )]);
    let opt_integer = define_struct(&mut m, "OptionInteger", &[(
        "vec",
        vector(struct_tok(integer)),
    )]);
    let optional_aggregator = define_struct(&mut m, "OptionalAggregator", &[
        ("aggregator", struct_tok(opt_aggregator)),
        ("integer", struct_tok(opt_integer)),
    ]);
    let opt_optional = define_struct(&mut m, "OptionOptionalAggregator", &[(
        "vec",
        vector(struct_tok(optional_aggregator)),
    )]);
    define_struct(&mut m, "CoinInfo", &[("supply", struct_tok(opt_optional))]);

    // --- PackageRegistry (0x1::code) — heaviest realistic API payload ---
    let string = struct_idx(&m, "String");
    let any = define_struct(&mut m, "Any", &[
        ("type_name", struct_tok(string)),
        ("data", vector(U8)),
    ]);
    let opt_any = define_struct(&mut m, "OptionAny", &[("vec", vector(struct_tok(any)))]);
    let upgrade_policy = define_struct(&mut m, "UpgradePolicy", &[("policy", U8)]);
    let module_metadata = define_struct(&mut m, "ModuleMetadata", &[
        ("name", struct_tok(string)),
        ("source", vector(U8)),
        ("source_map", vector(U8)),
        ("extension", struct_tok(opt_any)),
    ]);
    let package_dep = define_struct(&mut m, "PackageDep", &[
        ("account", Address),
        ("package_name", struct_tok(string)),
    ]);
    let package_metadata = define_struct(&mut m, "PackageMetadata", &[
        ("name", struct_tok(string)),
        ("upgrade_policy", struct_tok(upgrade_policy)),
        ("upgrade_number", U64),
        ("source_digest", struct_tok(string)),
        ("manifest", vector(U8)),
        ("modules", vector(struct_tok(module_metadata))),
        ("deps", vector(struct_tok(package_dep))),
        ("extension", struct_tok(opt_any)),
    ]);
    define_struct(&mut m, "PackageRegistry", &[(
        "packages",
        vector(struct_tok(package_metadata)),
    )]);

    // --- Layer 3b shapes: structural-edge / max-size workloads ---

    // WideStruct — 30 fields, the verifier's `max_fields_in_struct` = 30 wall. Field types come
    // from `wide_field` (which also produces the matching values), so type and value stay aligned.
    let wide_names: Vec<String> = (0..30).map(|i| format!("f{i:02}")).collect();
    let wide_fields: Vec<(&str, SignatureToken)> = wide_names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), wide_field(i).0))
        .collect();
    define_struct(&mut m, "WideStruct", &wide_fields);

    // BigEnum — 90 variants, the verifier's `max_struct_variants` = 90 wall. Each variant
    // carries the same two-field body; the value (`big_enum_elem`) selects the highest tag.
    let enum_vnames: Vec<String> = (0..90).map(|i| format!("V{i:02}")).collect();
    let big_enum_variants: Vec<(&str, Vec<(&str, SignatureToken)>)> = enum_vnames
        .iter()
        .map(|name| (name.as_str(), vec![("a", U64), ("b", Bool)]))
        .collect();
    define_enum(&mut m, "BigEnum", big_enum_variants);

    m
}

// ============================================================
// Layer 2: value helpers + per-shape element/payload builders
// ============================================================

/// `Object<T>`-shaped value: a struct `{ inner: address }`.
fn move_object(addr: AccountAddress) -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Address(addr)]))
}

/// `EventHandle { counter: u64, guid: GUID { id: ID { creation_num: u64, addr } } }`.
fn move_event_handle() -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(0),
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Struct(
            MoveStruct::Runtime(vec![
                MoveValue::U64(0),
                MoveValue::Address(AccountAddress::ONE),
            ]),
        )])),
    ]))
}

/// `0x1::string::String`-shaped value: a struct `{ bytes: vector<u8> }`.
fn move_string(s: &str) -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(
        s.bytes().map(MoveValue::U8).collect(),
    )]))
}

/// An empty `Option<Any>` value: `{ vec: vector<Any> }` of length 0.
fn none_any() -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(vec![])]))
}

/// One `FungibleStore { metadata: Object, balance: u64, frozen: bool }`.
fn fungible_store_elem() -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(vec![
        move_object(AccountAddress::ONE),
        MoveValue::U64(1_000),
        MoveValue::Bool(false),
    ]))
}

/// One `CoinStore { coin: Coin { value }, frozen, deposit_events, withdraw_events }`.
fn coin_store_elem() -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(500)])),
        MoveValue::Bool(false),
        move_event_handle(),
        move_event_handle(),
    ]))
}

/// One v2 `Token`: `Object` collection ref, an index, three `String`s, an `EventHandle`.
fn token_elem() -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(vec![
        move_object(AccountAddress::ONE),
        MoveValue::U64(1_234),
        move_string("a tokenized digital collectible with a medium-length description"),
        move_string("Token #1234"),
        move_string("https://example.com/nft/metadata/1234.json"),
        move_event_handle(),
    ]))
}

/// One `PropertyMap { inner: SimpleMap { data: vector<Element> } }` of `props`
/// string-keyed entries with `PropertyValue { type: u8, value: vector<u8> }` leaves.
fn property_map_elem(props: usize) -> MoveValue {
    let entry = MoveValue::Struct(MoveStruct::Runtime(vec![
        move_string("property_key_name"),
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U8(0x09),
            MoveValue::Vector((0..16u8).map(MoveValue::U8).collect()),
        ])),
    ]));
    MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Struct(
        MoveStruct::Runtime(vec![MoveValue::Vector(vec![entry; props])]),
    )]))
}

/// One `VersionedConfig` in its `V2` variant (tag 1):
/// `{ flag: bool, inner: AuditorShape, paused: bool }`.
fn versioned_v2_elem() -> MoveValue {
    MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![
        MoveValue::Bool(true),
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(
            (0..32u8).map(MoveValue::U8).collect(),
        )])),
        MoveValue::Bool(false),
    ]))
}

/// One `CoinInfo { supply: Option<OptionalAggregator> }` with the aggregator side
/// `Some` and the integer side `None`, exercising both 0- and 1-length wrappers.
fn coin_info_elem() -> MoveValue {
    let some_aggregator = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(vec![
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::Address(AccountAddress::ONE),
            MoveValue::Address(AccountAddress::ONE),
            MoveValue::U128(0),
        ])),
    ])]));
    let none_integer = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(vec![])]));
    let optional_aggregator =
        MoveValue::Struct(MoveStruct::Runtime(vec![some_aggregator, none_integer]));
    let supply = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(vec![
        optional_aggregator,
    ])]));
    MoveValue::Struct(MoveStruct::Runtime(vec![supply]))
}

/// Type and value of field `i` of `WideStruct`, as a single pair so the two projections
/// cannot drift (BCS is positional — a type/value mismatch would silently mis-decode). The
/// struct definition takes the `SignatureToken`; the payload takes the `MoveValue`; both come
/// from this one cycle, so the lockstep is structural, not comment-enforced.
fn wide_field(i: usize) -> (SignatureToken, MoveValue) {
    use SignatureToken::{Address, Bool, U128, U64};
    match i % 4 {
        0 => (U64, MoveValue::U64(i as u64)),
        1 => (Bool, MoveValue::Bool(i.is_multiple_of(2))),
        2 => (Address, MoveValue::Address(AccountAddress::ONE)),
        _ => (U128, MoveValue::U128(i as u128)),
    }
}

/// One `WideStruct`: 30 field values, each from `wide_field` so they match the struct's field
/// types position-for-position by construction.
fn wide_struct_elem() -> MoveValue {
    MoveValue::Struct(MoveStruct::Runtime(
        (0..30).map(|i| wide_field(i).1).collect(),
    ))
}

/// `vector<WideStruct>` filled to ~1 MB — pins the struct-field wall (30 fields) under the
/// storage-write cap. Reuses the canonical `vec_of` driver, then asserts the legal size.
///
/// Peak annotation heap: modest (~tens of MB) — well under the production 100 MB budget.
fn edge_struct_width_30() -> (ModuleMapView, Vec<u8>, TypeTag) {
    let elem = wide_struct_elem();
    let n = fill_count(&elem);
    let payload = vec_of("WideStruct", elem, n, None, 30);
    assert_legal_size(&payload.1);
    payload
}

/// One `BigEnum` at the highest tag (89): `{ a: u64, b: bool }`. The 90-variant *count* lives
/// in the type definition; the value is a single variant exercising the deepest tag dispatch.
fn big_enum_elem() -> MoveValue {
    MoveValue::Struct(MoveStruct::RuntimeVariant(89, vec![
        MoveValue::U64(0),
        MoveValue::Bool(true),
    ]))
}

/// `vector<BigEnum>` filled to ~1 MB — pins the enum-variant wall (90 variants), value at tag
/// 89. The only Layer-3b shape on the `RuntimeVariant` / `FatStructLayout::Variants` path.
///
/// Peak annotation heap: modest — well under the production 100 MB budget.
fn edge_enum_variants_90() -> (ModuleMapView, Vec<u8>, TypeTag) {
    let elem = big_enum_elem();
    let n = fill_count(&elem);
    let payload = vec_of("BigEnum", elem, n, Some(89), 2);
    assert_legal_size(&payload.1);
    payload
}

/// `vector<Chain>` of 128-deep nested vectors over a `u64` leaf, filled to ~1 MB — pins the
/// value-nesting wall (`DEFAULT_MAX_VM_VALUE_NESTED_DEPTH` = 128). Built directly against the
/// empty view (nested primitive vectors need no module lookup), like `nested_vec_u64`.
///
/// `u64` (not `u8`) leaf: a `vector<u8>` collapses to a `Bytes` node, erasing the innermost
/// level. Heaviest workload by far — every `Vector` node clones its element `TypeTag`, so this
/// legal ≤1 MB resource expands to ~1.5 GB of annotation heap and would abort under the
/// production 100 MB budget.
fn edge_value_depth_128() -> (ModuleMapView, Vec<u8>, TypeTag) {
    const DEPTH: usize = 128; // DEFAULT_MAX_VM_VALUE_NESTED_DEPTH

    // One chain: a u64 leaf wrapped in (DEPTH - 1) single-element vectors. The outer vector
    // added below contributes the final level, for DEPTH nested Vector nodes total.
    let mut chain = MoveValue::U64(0);
    for _ in 0..DEPTH - 1 {
        chain = MoveValue::Vector(vec![chain]);
    }
    let chain_bytes = bcs::to_bytes(&chain).expect("chain must serialize").len();
    let n = (MAX_RESOURCE_BYTES / chain_bytes.max(1)).max(1);
    let blob = bcs::to_bytes(&MoveValue::Vector(vec![chain; n])).unwrap();

    // Type: DEPTH nested vectors over u64.
    let mut ty = TypeTag::U64;
    for _ in 0..DEPTH {
        ty = TypeTag::Vector(Box::new(ty));
    }

    assert_legal_size(&blob);
    let view = ModuleMapView::empty();
    // Self-check: annotate once (metering OFF, so the budget never trips) and confirm the
    // realized depth is exactly 128.
    let annotator = MoveValueAnnotator::new_with_meter_config(&view, BUDGET, read_metering_off);
    let annotated = annotator
        .view_value(&ty, &blob)
        .expect("depth payload must annotate");
    assert_eq!(
        container_depth(&annotated),
        DEPTH,
        "value must nest to depth {DEPTH}"
    );

    (view, blob, ty)
}

/// Bare `vector<bool>` of ~1 M elements (~1 byte each), filled to ~1 MB — pins the node-count
/// wall: the most annotation nodes (hence `Meter::check()` reader calls) a single legal
/// resource can force. Built against the empty view, like `flat_vec_u64`.
///
/// `bool`, not `u8`: the annotator collapses `vector<u8>` to one `Bytes` node, which would
/// yield a single node; `bool` is 1 byte/element and is annotated per-element.
///
/// Peak annotation heap: ~150–170 MB (each bool expands to a full `AnnotatedMoveValue::Bool`
/// node) — crosses the production 100 MB budget, so it aborts under production metering.
fn edge_nodes_1mb() -> (ModuleMapView, Vec<u8>, TypeTag) {
    // Leave headroom for the vector length prefix so the blob stays under the 1 MB cap.
    let n = MAX_RESOURCE_BYTES - 16;
    let blob = bcs::to_bytes(&MoveValue::Vector(vec![MoveValue::Bool(false); n])).unwrap();
    assert_legal_size(&blob);
    (
        ModuleMapView::empty(),
        blob,
        TypeTag::Vector(Box::new(TypeTag::Bool)),
    )
}

/// A wide, shallow payload: `vector<u64>` with `n` elements. Lots of nodes, each
/// cheap to allocate — stresses the per-node reader call rather than allocation.
fn flat_vec_u64(n: usize) -> (ModuleMapView, Vec<u8>, TypeTag) {
    let blob = bcs::to_bytes(&MoveValue::Vector(vec![MoveValue::U64(0); n])).unwrap();
    (
        ModuleMapView::empty(),
        blob,
        TypeTag::Vector(Box::new(TypeTag::U64)),
    )
}

/// A deep, amplifying payload: `vector<vector<vector<u64>>>` of `dim^3` leaves.
fn nested_vec_u64(dim: usize) -> (ModuleMapView, Vec<u8>, TypeTag) {
    let inner = MoveValue::Vector(vec![MoveValue::U64(0); dim]);
    let mid = MoveValue::Vector(vec![inner; dim]);
    let outer = MoveValue::Vector(vec![mid; dim]);
    let blob = bcs::to_bytes(&outer).unwrap();
    let ty = TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(TypeTag::Vector(
        Box::new(TypeTag::U64),
    )))));
    (ModuleMapView::empty(), blob, ty)
}

/// `PackageRegistry` (`0x1::code`) — the heaviest realistic API payload: large
/// `vector<u8>` sources + `String` names, nested two levels. A single top-level
/// struct (not a vector), parameterized so it does not dominate total bench time.
fn package_registry(
    pkgs: usize,
    modules_per_pkg: usize,
    src_bytes: usize,
) -> (ModuleMapView, Vec<u8>, TypeTag) {
    let view = ModuleMapView::from_modules(vec![bench_module()]);

    let source: Vec<MoveValue> = (0..src_bytes).map(|i| MoveValue::U8(i as u8)).collect();
    let module_meta = MoveValue::Struct(MoveStruct::Runtime(vec![
        move_string("module_name"),
        MoveValue::Vector(source.clone()), // source (~src_bytes)
        MoveValue::Vector(source),         // source_map (~src_bytes)
        none_any(),                        // extension: Option<Any>
    ]));
    let dep = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Address(AccountAddress::ONE),
        move_string("AptosFramework"),
    ]));
    let package_meta = MoveValue::Struct(MoveStruct::Runtime(vec![
        move_string("MyPackage"),
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U8(1)])), // UpgradePolicy
        MoveValue::U64(1),                                              // upgrade_number
        move_string("0x0000000000000000000000000000000000000000000000000000000000000000"),
        MoveValue::Vector((0..256u32).map(|i| MoveValue::U8(i as u8)).collect()), // manifest
        MoveValue::Vector(vec![module_meta; modules_per_pkg]),
        MoveValue::Vector(vec![dep; 4]), // 4 framework deps (fixed, not parameterized)
        none_any(),
    ]));
    let registry = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(
        vec![package_meta; pkgs],
    )]));
    let blob = bcs::to_bytes(&registry).unwrap();
    let ty = struct_tag("PackageRegistry");

    check_struct(&view, &blob, &ty, 1);

    (view, blob, ty)
}

/// `Outer { items: [Inner; n], tag: [u8; 32], n: 0 }` plus the module defining
/// its types. Eagerly annotates once and asserts the shape, so a broken
/// module/blob panics at setup instead of silently mis-measuring.
///
/// Builds a single top-level `Outer` struct (not a `vector<Shape>`), so it uses
/// `check_struct` and a bespoke body rather than the `vec_of` driver — the same
/// deliberate odd-one-out pattern as `package_registry`.
fn vec_of_structs(n: usize) -> (ModuleMapView, Vec<u8>, TypeTag) {
    let view = ModuleMapView::from_modules(vec![bench_module()]);

    let inner = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(7),
        MoveValue::U64(8),
        MoveValue::Address(AccountAddress::ONE),
        MoveValue::Bool(true),
    ]));
    let outer = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Vector(vec![inner; n]),
        MoveValue::Vector((0..32u8).map(MoveValue::U8).collect()),
        MoveValue::U128(0),
    ]));
    let blob = bcs::to_bytes(&outer).unwrap();
    let ty = struct_tag("Outer");

    check_struct(&view, &blob, &ty, 3);

    (view, blob, ty)
}

// ============================================================
// Layer 3: generic driver, self-checks, and the workload table
// ============================================================

/// Builds a `vector<Shape>` workload of `n` copies of `elem`, where `Shape` is
/// `0x1::bench::<name>`, and self-checks it (`variant`/`fields` describe the first
/// element). Returns the `(view, blob, ty)` triple the bench loop consumes. This is
/// the single place the from-modules / bcs / tag / self-check ritual lives.
fn vec_of(
    name: &str,
    elem: MoveValue,
    n: usize,
    variant: Option<u16>,
    fields: usize,
) -> (ModuleMapView, Vec<u8>, TypeTag) {
    let view = ModuleMapView::from_modules(vec![bench_module()]);
    let blob = bcs::to_bytes(&MoveValue::Vector(vec![elem; n])).unwrap();
    let ty = TypeTag::Vector(Box::new(struct_tag(name)));
    check_vector(&view, &blob, &ty, variant, fields);
    (view, blob, ty)
}

/// Setup-time self-check: annotate `blob` as `vector<struct>` and assert its first
/// element annotates as variant `variant` (`None` for a plain struct) with `fields`
/// fields. Panics at setup on a malformed shape.
fn check_vector(
    view: &ModuleMapView,
    blob: &[u8],
    ty: &TypeTag,
    variant: Option<u16>,
    fields: usize,
) {
    let annotator = MoveValueAnnotator::new_with_meter_config(view, BUDGET, read_metering_off);
    match annotator
        .view_value(ty, blob)
        .expect("bench payload must annotate")
    {
        AnnotatedMoveValue::Vector(_, elems) => {
            let s = match elems.first() {
                Some(AnnotatedMoveValue::Struct(s)) => s,
                other => panic!("expected a struct element, got {other:?}"),
            };
            assert_eq!(
                s.variant_info.as_ref().map(|(t, _)| *t),
                variant,
                "element variant mismatch",
            );
            assert_eq!(
                s.value.len(),
                fields,
                "element must annotate to {fields} fields"
            );
        },
        other => panic!("expected an annotated vector, got {other:?}"),
    }
}

/// Setup-time self-check: annotate `blob` as a top-level struct and assert it has
/// `fields` fields. Panics at setup (not mid-measure) on a malformed shape.
fn check_struct(view: &ModuleMapView, blob: &[u8], ty: &TypeTag, fields: usize) {
    let annotator = MoveValueAnnotator::new_with_meter_config(view, BUDGET, read_metering_off);
    match annotator
        .view_value(ty, blob)
        .expect("bench struct payload must annotate")
    {
        AnnotatedMoveValue::Struct(s) => {
            assert_eq!(
                s.value.len(),
                fields,
                "struct must annotate to {fields} fields"
            );
        },
        other => panic!("expected an annotated struct, got {other:?}"),
    }
}

/// Realized nesting depth of an annotated value, counting `Vector` containers (the leaf is not
/// a container). Used by `edge_value_depth_128`'s self-check to confirm the value reaches the
/// `DEFAULT_MAX_VM_VALUE_NESTED_DEPTH` = 128 ceiling. The `_` arm is intentional: for this
/// payload every non-`Vector` value is genuinely a depth-0 leaf, not a silenced enum variant.
fn container_depth(v: &AnnotatedMoveValue) -> usize {
    match v {
        AnnotatedMoveValue::Vector(_, elems) => 1 + elems.first().map_or(0, container_depth),
        _ => 0,
    }
}

/// Repeat count so a `vector<elem>` payload lands just under the 1 MB storage-write cap.
/// `elem` is serialized once for its exact BCS size; the vector length prefix (≤ 9 bytes) is
/// negligible.
fn fill_count(elem: &MoveValue) -> usize {
    let elem_bytes = bcs::to_bytes(elem).expect("element must serialize").len();
    (MAX_RESOURCE_BYTES / elem_bytes.max(1)).max(1)
}

/// Setup-time assertion that an edge payload is a *legal, genuinely-maxed* resource: at most
/// the 1 MB write cap (could exist on-chain) and at least 0.9 MB (actually stresses the
/// limit, not a toy).
fn assert_legal_size(blob: &[u8]) {
    assert!(
        blob.len() <= MAX_RESOURCE_BYTES,
        "edge payload is {} bytes, over the {MAX_RESOURCE_BYTES}-byte storage-write cap",
        blob.len(),
    );
    assert!(
        blob.len() * 10 >= MAX_RESOURCE_BYTES * 9,
        "edge payload is {} bytes, under 0.9 MB — not genuinely maxed",
        blob.len(),
    );
}

fn annotate_once(view: &ModuleMapView, blob: &[u8], ty: &TypeTag, read_live: fn() -> i64) {
    let annotator = MoveValueAnnotator::new_with_meter_config(view, BUDGET, read_live);
    let value = annotator
        .view_value(ty, blob)
        .expect("benchmark payload must annotate within budget");
    black_box(value);
}

/// The single workload table shared by the timing suite and the amplification report. String
/// literals give `&'static str` names; each tuple is the `(view, blob, type)` an annotation
/// consumes.
fn workloads() -> Vec<(&'static str, (ModuleMapView, Vec<u8>, TypeTag))> {
    vec![
        ("flat_vec_u64_80k", flat_vec_u64(80_000)),
        ("nested_vec_u64_48", nested_vec_u64(48)),
        ("vec_of_structs_20k", vec_of_structs(20_000)),
        (
            "vec_fungible_store_20k",
            vec_of("FungibleStore", fungible_store_elem(), 20_000, None, 3),
        ),
        (
            "vec_coin_store_10k",
            vec_of("CoinStore", coin_store_elem(), 10_000, None, 4),
        ),
        (
            "vec_token_10k",
            vec_of("Token", token_elem(), 10_000, None, 6),
        ),
        (
            "vec_property_map_2k_x16",
            vec_of("PropertyMap", property_map_elem(16), 2_000, None, 1),
        ),
        (
            "vec_versioned_enum_20k",
            vec_of("VersionedConfig", versioned_v2_elem(), 20_000, Some(1), 3),
        ),
        (
            "vec_coin_info_10k",
            vec_of("CoinInfo", coin_info_elem(), 10_000, None, 1),
        ),
        ("package_registry_4x8x800", package_registry(4, 8, 800)),
        // --- Layer 3b: structural-edge / max-size workloads (design spec 2026-06-24) ---
        ("edge_struct_width_30", edge_struct_width_30()),
        ("edge_enum_variants_90", edge_enum_variants_90()),
        ("edge_value_depth_128", edge_value_depth_128()),
        ("edge_nodes_1mb", edge_nodes_1mb()),
    ]
}

fn bench_annotation(c: &mut Criterion, workloads: &[(&str, (ModuleMapView, Vec<u8>, TypeTag))]) {
    for (name, (view, blob, ty)) in workloads {
        let mut group = c.benchmark_group(*name);
        group.bench_function("metering_off", |b| {
            b.iter(|| annotate_once(view, blob, ty, read_metering_off))
        });
        group.bench_function("metering_on", |b| {
            b.iter(|| annotate_once(view, blob, ty, aptos_jemalloc::current_live_bytes))
        });
        group.finish();
    }
}

/// Production per-request annotation budget (the real config default).
const PROD_BUDGET: usize = DEFAULT_MAX_RESOURCE_ANNOTATION_BYTES; // 100 MB
/// Max concurrent annotations: the API blocking-pool cap (`MAX_BLOCKING_THREADS`,
/// `crates/aptos-runtimes/src/lib.rs`). Annotation runs on `api_spawn_blocking`.
const ANNOTATION_CONCURRENCY: usize = 64;
/// Assumed node-wide memory tolerance, for the implied-budget calculation.
const MEMORY_TOLERANCE: usize = 20_000_000_000; // 20 GB

/// One-shot heap-amplification characterization, printed before the timing suite. For each
/// workload it annotates once with the real jemalloc reader, measures peak live heap, and
/// prints disk size, peak heap, the disk→heap amplification factor, and whether the payload
/// would abort under the production per-request budget. The realistic shapes stay single-digit
/// × and never abort; only the pathological `edge_value_depth_128` (~1500×) aborts.
fn report_amplification(workloads: &[(&str, (ModuleMapView, Vec<u8>, TypeTag))]) {
    println!("\n=== annotation heap-amplification report ===");
    println!(
        "per-request budget {} MB · concurrency {} · aggregate worst {} GB · no global cap",
        PROD_BUDGET / 1_000_000,
        ANNOTATION_CONCURRENCY,
        PROD_BUDGET * ANNOTATION_CONCURRENCY / 1_000_000_000,
    );
    println!(
        "implied safe per-request budget at {} GB tolerance: {} MB",
        MEMORY_TOLERANCE / 1_000_000_000,
        MEMORY_TOLERANCE / ANNOTATION_CONCURRENCY / 1_000_000,
    );
    println!(
        "{:<28} {:>10} {:>11} {:>8}  aborts@budget?",
        "workload", "disk", "peak heap", "amp"
    );
    for (name, (view, blob, ty)) in workloads {
        let before = aptos_jemalloc::current_live_bytes();
        let annotator = MoveValueAnnotator::new_with_meter_config(
            view,
            BUDGET,
            aptos_jemalloc::current_live_bytes,
        );
        let annotated = annotator
            .view_value(ty, blob)
            .expect("report payload must annotate within BUDGET");
        let peak = (aptos_jemalloc::current_live_bytes() - before).max(0) as usize;
        black_box(&annotated);
        drop(annotated);
        let amp = peak as f64 / blob.len().max(1) as f64;
        println!(
            "{:<28} {:>9}K {:>10}M {:>7.0}x  {}",
            name,
            blob.len() / 1_000,
            peak / 1_000_000,
            amp,
            if peak > PROD_BUDGET { "YES" } else { "no" },
        );
    }
    println!();
}

fn main() {
    let workloads = workloads();
    report_amplification(&workloads);
    let mut criterion = Criterion::default().configure_from_args();
    bench_annotation(&mut criterion, &workloads);
    criterion.final_summary();
}

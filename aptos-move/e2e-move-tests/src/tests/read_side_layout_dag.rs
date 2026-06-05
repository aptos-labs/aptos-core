// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Read-side `MoveTypeLayout` construction from small, verifier-legal types whose layout has
//! exponentially many nodes when shared instantiations are expanded instead of reused. Reached
//! through the two off-chain builders the REST API uses.

use crate::{assert_success, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_resource_viewer::{AnnotatedMoveValue, AptosValueAnnotator};
use aptos_types::account_address::AccountAddress;
use claims::{assert_err, assert_ok};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};

const DEPTH: usize = 8;
const FAN_OUT: usize = 16;

/// A module with a generic `struct Wide<T>` of `FAN_OUT` fields, each of type `T`.
fn wide_module_source(addr: &str) -> String {
    let mut s = format!("module {}::wide {{\n", addr);
    s.push_str("    struct Wide<T> has copy, drop, store {\n");
    for i in 0..FAN_OUT {
        s.push_str(&format!("        f{}: T,\n", i));
    }
    s.push_str("    }\n}\n");
    s
}

/// `vector<Wide<Wide<...<u8>>>>` with `DEPTH` levels of `Wide`.
fn nested_wide_vector(addr: AccountAddress) -> TypeTag {
    let mut inner = TypeTag::U8;
    for _ in 0..DEPTH {
        inner = TypeTag::Struct(Box::new(StructTag {
            address: addr,
            module: Identifier::new("wide").unwrap(),
            name: Identifier::new("Wide").unwrap(),
            type_args: vec![inner],
        }));
    }
    TypeTag::Vector(Box::new(inner))
}

/// `view_value` shares repeated struct instantiations instead of expanding them into a tree.
#[test]
fn view_value_bounds_layout_dag() {
    let addr = "0xcafe";
    let account_address = AccountAddress::from_hex_literal(addr).unwrap();

    let mut builder = PackageBuilder::new("Wide");
    builder.add_source("wide.move", &wide_module_source(addr));
    let path = builder.write_to_temp().unwrap();

    let mut h = MoveHarness::new();
    let acc = h.new_account_at(account_address);
    assert_success!(h.publish_package(&acc, path.path()));

    let type_tag = nested_wide_vector(account_address);
    let empty_vector = bcs::to_bytes(&Vec::<u8>::new()).unwrap();

    let annotator = AptosValueAnnotator::new(h.executor.state_store());
    let value = assert_ok!(annotator.view_value(&type_tag, &empty_vector));

    match value {
        AnnotatedMoveValue::Vector(_, elems) => assert!(elems.is_empty()),
        other => panic!("expected an empty vector, got {:?}", other),
    }
}

/// A concrete-width chain `L0 { v: u8 }`, `L(i+1) { f0..f15: Li }`. With no type parameters to
/// share, `TypeLayoutBuilder` rebuilds each field, so `Li`'s layout has `16^i` nodes.
fn concrete_chain_source(addr: &str, depth: usize) -> String {
    let mut s = format!("module {}::nest {{\n", addr);
    s.push_str("    struct L0 has copy, drop, store { v: u8 }\n");
    for i in 0..depth {
        s.push_str(&format!("    struct L{} has copy, drop, store {{\n", i + 1));
        for f in 0..FAN_OUT {
            s.push_str(&format!("        f{}: L{},\n", f, i));
        }
        s.push_str("    }\n");
    }
    s.push_str("}\n");
    s
}

fn l_struct_tag(addr: AccountAddress, level: usize) -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: addr,
        module: Identifier::new("nest").unwrap(),
        name: Identifier::new(format!("L{}", level)).unwrap(),
        type_args: vec![],
    }))
}

/// `view_fully_decorated_ty_layout` caps the layout's node count.
#[test]
fn view_fully_decorated_ty_layout_bounds_layout_dag() {
    let addr = "0xcafe";
    let account_address = AccountAddress::from_hex_literal(addr).unwrap();

    let mut builder = PackageBuilder::new("Nest");
    builder.add_source("nest.move", &concrete_chain_source(addr, 4));
    let path = builder.write_to_temp().unwrap();

    let mut h = MoveHarness::new();
    let acc = h.new_account_at(account_address);
    assert_success!(h.publish_package(&acc, path.path()));

    let annotator = AptosValueAnnotator::new(h.executor.state_store());

    // L1's layout is 16 nodes, well under the cap.
    assert_ok!(annotator.view_fully_decorated_ty_layout(&l_struct_tag(account_address, 1)));
    // L4's is 16^4, over the cap.
    assert_err!(annotator.view_fully_decorated_ty_layout(&l_struct_tag(account_address, 4)));
}

/// The layout node cap must match the VM's runtime `layout_max_size`.
#[test]
fn layout_node_cap_matches_vm_limit() {
    assert_eq!(
        move_bytecode_utils::layout::MAX_TYPE_LAYOUT_NODES,
        move_vm_runtime::config::VMConfig::default().layout_max_size,
    );
}

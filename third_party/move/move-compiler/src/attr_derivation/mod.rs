// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    attr_derivation::{
        async_deriver::{add_attributes_for_async, derive_for_async},
        evm_deriver::{add_attributes_for_evm, derive_for_evm},
    },
    parser::ast::{
        Attribute, AttributeValue, Attribute_, Attributes, Definition, Exp, Exp_, Function,
        FunctionBody_, FunctionName, FunctionSignature, LeadingNameAccess_, NameAccessChain,
        NameAccessChain_, StructDefinition, StructFields, StructName, Type, Type_, Value_, Var,
        Visibility,
    },
    shared::{
        known_attributes::{AttributeKind, KnownAttribute},
        CompilationEnv, Flags, Name, NamedAddressMap,
    },
};
use move_ir_types::location::{sp, Loc};
use move_symbol_pool::Symbol;
use std::collections::BTreeSet;

mod async_deriver;
mod evm_deriver;

const EVM_FLAVOR: &str = "evm";
const ASYNC_FLAVOR: &str = "async";

const EVENT_ATTR: &str = "event";

/// Entry point for deriving definitions from attributes for the given module. Depending on the
/// flavor specified via the flags, this is dispatching to the according attribute processor.
pub fn derive_from_attributes(
    env: &mut CompilationEnv,
    address_map: &NamedAddressMap,
    def: &mut Definition,
) {
    if env.flags().has_flavor(EVM_FLAVOR) {
        derive_for_evm(env, address_map, def)
    }
    if env.flags().has_flavor(ASYNC_FLAVOR) {
        derive_for_async(env, address_map, def)
    }
}

pub fn add_attributes_for_flavor(flags: &Flags, known_attributes: &mut BTreeSet<String>) {
    if flags.has_flavor(EVM_FLAVOR) {
        add_attributes_for_evm(known_attributes);
    }
    if flags.has_flavor(ASYNC_FLAVOR) {
        add_attributes_for_async(known_attributes);
        // Tests with flavor "async" seem to also use EVM attributes.
        add_attributes_for_evm(known_attributes);
    }
    KnownAttribute::add_attribute_names(known_attributes);
}

pub fn get_known_attributes_for_flavor(flags: &Flags) -> BTreeSet<String> {
    let mut known_attributes = BTreeSet::new();
    add_attributes_for_flavor(flags, &mut known_attributes);
    known_attributes
}

// ==========================================================================================
// Helper Functions for analyzing attributes and creating the AST

/// Helper function to find an attribute by name.
pub(crate) fn find_attr<'a>(attrs: &'a Attributes, name: &str) -> Option<&'a Attribute> {
    attrs
        .value
        .iter()
        .find(|a| a.value.attribute_name().value.as_str() == name)
}

/// Helper function to find an attribute in a slice.
pub(crate) fn find_attr_slice<'a>(vec: &'a [Attributes], name: &str) -> Option<&'a Attribute> {
    for attrs in vec {
        if let Some(a) = find_attr(attrs, name) {
            return Some(a);
        }
    }
    None
}

/// Helper to extract the parameters of an attribute. If the attribute is of the form
/// `n(a1, ..., an)`, this extracts the a_i as a vector. Otherwise the attribute is assumed
/// to have no parameters.
pub(crate) fn attr_params(attr: &Attribute) -> Vec<&Attribute> {
    match &attr.value {
        Attribute_::Parameterized(_, vs) => vs.value.iter().collect(),
        _ => vec![],
    }
}

/// Helper to extract a named value attribute, as in `n [= v]`.
#[allow(unused)]
pub(crate) fn attr_value(attr: &Attribute) -> Option<(&Name, Option<&AttributeValue>)> {
    match &attr.value {
        Attribute_::Name(n) => Some((n, None)),
        Attribute_::Assigned(n, v) => Some((n, Some(v))),
        _ => None,
    }
}

/// Creates a new attribute.
pub(crate) fn new_attr(loc: Loc, name: &str, params: Vec<Attribute>) -> Attribute {
    let n = sp(loc, Symbol::from(name));
    if params.is_empty() {
        sp(loc, Attribute_::Name(n))
    } else {
        sp(loc, Attribute_::Parameterized(n, sp(loc, params)))
    }
}

/// Helper to create a new native function declaration.
pub(crate) fn new_native_fun(
    loc: Loc,
    name: FunctionName,
    attributes: Attributes,
    visibility: Visibility,
    entry: Option<Loc>,
    signature: FunctionSignature,
) -> Function {
    Function {
        attributes: vec![attributes],
        loc,
        visibility,
        entry,
        signature,
        acquires: vec![],
        name,
        inline: false,
        body: sp(loc, FunctionBody_::Native),
    }
}

/// Helper to create a new function declaration.
pub(crate) fn new_fun(
    loc: Loc,
    name: FunctionName,
    attributes: Attributes,
    visibility: Visibility,
    entry: Option<Loc>,
    signature: FunctionSignature,
    def: Exp,
) -> Function {
    Function {
        attributes: vec![attributes],
        loc,
        visibility,
        entry,
        signature,
        acquires: vec![],
        name,
        inline: false,
        body: sp(
            loc,
            FunctionBody_::Defined((vec![], vec![], None, Box::new(Some(def)))),
        ),
    }
}

/// Helper to create a new struct declaration.
pub(crate) fn new_struct(loc: Loc, name: StructName, fields: StructFields) -> StructDefinition {
    StructDefinition {
        attributes: vec![sp(
            // #[event]
            loc,
            vec![new_attr(loc, EVENT_ATTR, vec![])],
        )],
        loc,
        abilities: vec![],
        name,
        type_parameters: vec![],
        fields,
    }
}

/// Helper to create a new named variable.
pub(crate) fn new_var(loc: Loc, name: &str) -> Var {
    Var(sp(loc, Symbol::from(name)))
}

/// Helper to create a new type, based on its simple name.
pub(crate) fn new_simple_type(loc: Loc, ty_str: &str, ty_args: Vec<Type>) -> Type {
    sp(
        loc,
        Type_::Apply(Box::new(new_simple_name(loc, ty_str)), ty_args),
    )
}

/// Helper to create a simple name.
pub(crate) fn new_simple_name(loc: Loc, name: &str) -> NameAccessChain {
    sp(loc, NameAccessChain_::One(sp(loc, Symbol::from(name))))
}

/// Helper to create a full name.
pub(crate) fn new_full_name(
    loc: Loc,
    addr_alias: &str,
    module: &str,
    name: &str,
) -> NameAccessChain {
    let leading = sp(
        loc,
        LeadingNameAccess_::Name(sp(loc, Symbol::from(addr_alias))),
    );
    sp(
        loc,
        NameAccessChain_::Three(
            sp(loc, (leading, sp(loc, Symbol::from(module)))),
            sp(loc, Symbol::from(name)),
        ),
    )
}

/// Helper to create a call exp.
pub(crate) fn new_call_exp(loc: Loc, fun: NameAccessChain, args: Vec<Exp>) -> Exp {
    sp(loc, Exp_::Call(fun, false, None, sp(loc, args)))
}

pub(crate) fn new_borrow_exp(loc: Loc, arg: Exp) -> Exp {
    sp(loc, Exp_::Borrow(false, Box::new(arg)))
}

/// Helper to create a name exp.
pub(crate) fn new_simple_name_exp(loc: Loc, name: Name) -> Exp {
    sp(loc, Exp_::Name(sp(loc, NameAccessChain_::One(name)), None))
}

/// Helper to create an expression for denoting a vector<u8> value.
#[allow(unused)]
pub(crate) fn new_vec_u8(loc: Loc, vec: &[u8]) -> Exp {
    let values = vec
        .iter()
        .map(|x| {
            sp(
                loc,
                Exp_::Value(sp(loc, Value_::Num(Symbol::from(x.to_string())))),
            )
        })
        .collect();
    sp(
        loc,
        Exp_::Vector(
            loc,
            Some(vec![new_simple_type(loc, "u8", vec![])]),
            sp(loc, values),
        ),
    )
}

/// Helper to create new u64.
pub(crate) fn new_u64(loc: Loc, val: u64) -> Exp {
    sp(
        loc,
        Exp_::Value(sp(loc, Value_::Num(Symbol::from(val.to_string())))),
    )
}

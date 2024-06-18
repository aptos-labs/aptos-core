// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loaded_data::{
    runtime_access_specifier::{
        AccessInstance, AccessSpecifier, AccessSpecifierClause, AddressSpecifier, ResourceSpecifier,
    },
    runtime_types::{StructIdentifier, Type, TypeBuilder},
};
use move_binary_format::file_format::AccessKind;
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig{cases: 5000, verbose: 1, ..ProptestConfig::default()})]

    /// Test in all combinations the semi-lattice properties of the join and subsumes.
    /// Together with testing basic membership, this gives extensive coverage of the operators.
    #[test]
    fn access_specifier_semi_lattice_properties(
        access in access_instance_strategy(),
        s1 in access_specifier_strategy(4, 3),
        s2 in access_specifier_strategy(4, 3)
    ) {
        if s1.enables(&access) && s2.enables(&access) {
            assert!(s1.join(&s2).enables(&access))
        } else {
            assert!(!s1.join(&s2).enables(&access))
        }
        if s1.subsumes(&s2).unwrap_or_default() && s2.enables(&access) {
            assert!(s1.enables(&access))
        }
    }

    /// Test membership, by constructing all combinations of specifiers derivable from a given
    /// instance.
    #[test]
    fn access_specifier_enables(
        (access1, clause1) in  access_to_matching_specifier_clause(access_instance_strategy()),
        (access2, clause2) in  access_to_matching_specifier_clause(access_instance_strategy()),
    ) {
        let clauses = vec![clause1, clause2];
        let incl = AccessSpecifier::Constraint(clauses.clone(), vec![]);
        let incl_excl = AccessSpecifier::Constraint(clauses.clone(), clauses.clone());
        let excl = AccessSpecifier::Constraint(vec![], clauses.clone());
        assert!(incl.enables(&access1));
        assert!(incl.enables(&access2));
        assert!(incl.join(&incl).enables(&access1));
        assert!(incl.join(&incl).enables(&access2));
        assert!(!incl_excl.enables(&access1));
        assert!(!incl_excl.enables(&access2));
        assert!(!excl.enables(&access1));
        assert!(!excl.enables(&access2));
    }
}

fn access_instance_strategy() -> impl Strategy<Value = AccessInstance> {
    (
        any::<AccessKind>(),
        struct_id_strategy(),
        type_args_strategy(),
        address_strategy(),
    )
        .prop_map(|(kind, resource, instance, address)| AccessInstance {
            kind,
            resource,
            instance,
            address,
        })
}
fn access_specifier_strategy(
    incl_size: usize,
    excl_size: usize,
) -> impl Strategy<Value = AccessSpecifier> {
    prop_oneof![
        Just(AccessSpecifier::Any),
        (
            vec(access_specifier_clause_strategy(), 0..incl_size),
            vec(access_specifier_clause_strategy(), 0..excl_size),
        )
            .prop_map(|(incls, excls)| AccessSpecifier::Constraint(incls, excls))
    ]
}

fn access_specifier_clause_strategy() -> impl Strategy<Value = AccessSpecifierClause> {
    (
        any::<AccessKind>(),
        resource_specifier_strategy(),
        address_specifier_strategy(),
    )
        .prop_map(|(kind, resource, address)| AccessSpecifierClause {
            kind,
            resource,
            address,
        })
}

fn resource_specifier_strategy() -> impl Strategy<Value = ResourceSpecifier> {
    prop_oneof![
        Just(ResourceSpecifier::Any),
        address_strategy().prop_map(ResourceSpecifier::DeclaredAtAddress),
        module_id_strategy().prop_map(ResourceSpecifier::DeclaredInModule),
        struct_id_strategy().prop_map(ResourceSpecifier::Resource),
        (struct_id_strategy(), type_args_strategy())
            .prop_map(|(s, ts)| ResourceSpecifier::ResourceInstantiation(s, ts)),
    ]
}

fn address_specifier_strategy() -> impl Strategy<Value = AddressSpecifier> {
    prop_oneof![
        Just(AddressSpecifier::Any),
        address_strategy().prop_map(AddressSpecifier::Literal) // Skip Eval as it is not appearing subsumes and join
    ]
}

fn type_args_strategy() -> impl Strategy<Value = Vec<Type>> {
    // Actual type builder limits do not matter because creating primitive
    // integer types is always possible.
    let ty_builder = TypeBuilder::with_limits(10, 10);
    prop_oneof![
        Just(vec![]),
        Just(vec![ty_builder.create_u8_ty()]),
        Just(vec![ty_builder.create_u16_ty(), ty_builder.create_u32_ty()])
    ]
}

fn struct_id_strategy() -> impl Strategy<Value = StructIdentifier> {
    (module_id_strategy(), identifier_strategy())
        .prop_map(|(module, name)| StructIdentifier { module, name })
}

fn module_id_strategy() -> impl Strategy<Value = ModuleId> {
    (address_strategy(), identifier_strategy()).prop_map(|(a, i)| ModuleId::new(a, i))
}

fn identifier_strategy() -> impl Strategy<Value = Identifier> {
    "[a-b]{1}[c-d]{1}".prop_map(|s| Identifier::new(s).unwrap())
}

fn address_strategy() -> impl Strategy<Value = AccountAddress> {
    prop_oneof![
        Just(AccountAddress::from_str_strict("0x1").unwrap()),
        Just(AccountAddress::from_str_strict("0x2").unwrap()),
        Just(AccountAddress::from_str_strict("0x3").unwrap())
    ]
}

/// Map a strategy of instances to matching access specifier clauses.
fn access_to_matching_specifier_clause(
    instances: impl Strategy<Value = AccessInstance>,
) -> impl Strategy<Value = (AccessInstance, AccessSpecifierClause)> {
    instances.prop_flat_map(|inst| {
        (
            Just(inst.kind),
            resource_to_matching_specifier(Just((inst.resource.clone(), inst.instance.clone()))),
            address_to_matching_specifier(Just(inst.address)),
        )
            .prop_map(move |(kind, resource, address)| {
                (inst.clone(), AccessSpecifierClause {
                    kind,
                    resource,
                    address,
                })
            })
    })
}

/// Map a strategy of resources to a strategy of specifiers which match them.
fn resource_to_matching_specifier(
    resources: impl Strategy<Value = (StructIdentifier, Vec<Type>)>,
) -> impl Strategy<Value = ResourceSpecifier> {
    resources.prop_flat_map(|(s, ts)| {
        prop_oneof![
            Just(ResourceSpecifier::Any),
            Just(ResourceSpecifier::DeclaredAtAddress(s.module.address)),
            Just(ResourceSpecifier::DeclaredInModule(s.module.clone())),
            Just(ResourceSpecifier::Resource(s.clone())),
            Just(ResourceSpecifier::ResourceInstantiation(s, ts))
        ]
    })
}

/// Map a strategy of addresses to a strategy of specifiers which match them.
fn address_to_matching_specifier(
    addresses: impl Strategy<Value = AccountAddress>,
) -> impl Strategy<Value = AddressSpecifier> {
    addresses.prop_flat_map(|a| {
        prop_oneof![
            Just(AddressSpecifier::Any),
            Just(AddressSpecifier::Literal(a))
        ]
    })
}

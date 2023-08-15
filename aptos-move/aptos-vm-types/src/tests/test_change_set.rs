// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    change_set::VMChangeSet,
    tests::utils::{
        build_change_set, mock_add, mock_create, mock_delete, mock_modify, MockChangeSetChecker,
    },
};
use aptos_types::{
    access_path::AccessPath,
    state_store::state_key::StateKey,
    transaction::ChangeSet as StorageChangeSet,
    write_set::{WriteOp, WriteSetMut},
};
use claims::{assert_matches, assert_ok};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{ModuleId, StructTag},
    vm_status::{StatusCode, VMStatus},
};
use std::collections::BTreeMap;

/// Testcases:
/// ```text
/// *--------------*----------------*----------------*-----------------*
/// |   state key  |  change set 1  |  change set 2  |    squashed     |
/// +--------------*----------------*----------------*-----------------*
/// |      0       |    create 0    |                |    create 0     |
/// |      1       |    modify 1    |                |    modify 1     |
/// |      2       |    delete      |                |    delete       |
/// |      3       |                |    create 103  |    create 103   |
/// |      4       |                |    modify 104  |    modify 104   |
/// |      5       |                |    delete      |    delete       |
/// |      6       |    create 6    |    create 106  |    ERROR        |
/// |      7       |    create 7    |    modify 107  |    create 107   |
/// |      8       |    create 8    |    delete      |                 |
/// |      9       |    modify 9    |    create 109  |    ERROR        |
/// |      10      |    modify 10   |    modify 110  |    modify 110   |
/// |      11      |    modify 11   |    delete      |    delete       |
/// |      12      |    delete      |    create 112  |    modify 112   |
/// |      13      |    delete      |    modify 113  |    ERROR        |
/// |      14      |    delete      |    delete      |    ERROR        |
/// *--------------*----------------*----------------*-----------------*
/// |      15      |    +15         |                |    +15          |
/// |      16      |                |    +116        |    +116         |
/// |      17      |    +17         |    +117        |    +134         |
/// *--------------*----------------*----------------*-----------------*
/// |      18      |    create 18   |    +118        |    create 136   |
/// |      19      |    modify 19   |    +119        |    modify 138   |
/// |      20      |    delete      |    +120        |    ERROR        |
/// *--------------*----------------*----------------*-----------------*
/// |      21      |    +21         |    create 121  |    ERROR        |
/// |      22      |    +22         |    modify 122  |    modify 122   |
/// |      23      |    +23         |    delete      |    delete       |
/// *--------------*----------------*----------------*-----------------*
/// ```

macro_rules! write_set_1 {
    ($d:ident) => {
        vec![
            mock_create(format!("0{}", $d), 0),
            mock_modify(format!("1{}", $d), 1),
            mock_delete(format!("2{}", $d)),
            mock_create(format!("7{}", $d), 7),
            mock_create(format!("8{}", $d), 8),
            mock_modify(format!("10{}", $d), 10),
            mock_modify(format!("11{}", $d), 11),
            mock_delete(format!("12{}", $d)),
        ]
    };
}

macro_rules! write_set_2 {
    ($d:ident) => {
        vec![
            mock_create(format!("3{}", $d), 103),
            mock_modify(format!("4{}", $d), 104),
            mock_delete(format!("5{}", $d)),
            mock_modify(format!("7{}", $d), 107),
            mock_delete(format!("8{}", $d)),
            mock_modify(format!("10{}", $d), 110),
            mock_delete(format!("11{}", $d)),
            mock_create(format!("12{}", $d), 112),
        ]
    };
}

macro_rules! expected_write_set {
    ($d:ident) => {
        BTreeMap::from([
            mock_create(format!("0{}", $d), 0),
            mock_modify(format!("1{}", $d), 1),
            mock_delete(format!("2{}", $d)),
            mock_create(format!("3{}", $d), 103),
            mock_modify(format!("4{}", $d), 104),
            mock_delete(format!("5{}", $d)),
            mock_create(format!("7{}", $d), 107),
            mock_modify(format!("10{}", $d), 110),
            mock_delete(format!("11{}", $d)),
            mock_modify(format!("12{}", $d), 112),
        ])
    };
}

// Populate sets according to the spec. Skip keys which lead to
// errors because we test them separately.
fn build_change_sets_for_test() -> (VMChangeSet, VMChangeSet) {
    let mut descriptor = "r";
    let resource_write_set_1 = write_set_1!(descriptor);
    descriptor = "m";
    let module_write_set_1 = write_set_1!(descriptor);
    let aggregator_write_set_1 = vec![mock_create("18a", 18), mock_modify("19a", 19)];
    let aggregator_delta_set_1 = vec![
        mock_add("15a", 15),
        mock_add("17a", 17),
        mock_add("22a", 22),
        mock_add("23a", 23),
    ];
    let change_set_1 = build_change_set(
        resource_write_set_1,
        module_write_set_1,
        aggregator_write_set_1,
        aggregator_delta_set_1,
    );

    descriptor = "r";
    let resource_write_set_2 = write_set_2!(descriptor);
    descriptor = "m";
    let module_write_set_2 = write_set_2!(descriptor);
    let aggregator_write_set_2 = vec![mock_modify("22a", 122), mock_delete("23a")];
    let aggregator_delta_set_2 = vec![
        mock_add("16a", 116),
        mock_add("17a", 117),
        mock_add("18a", 118),
        mock_add("19a", 119),
    ];
    let change_set_2 = build_change_set(
        resource_write_set_2,
        module_write_set_2,
        aggregator_write_set_2,
        aggregator_delta_set_2,
    );

    (change_set_1, change_set_2)
}

#[test]
fn test_successful_squash() {
    let (mut change_set, additional_change_set) = build_change_sets_for_test();
    assert_ok!(
        change_set.squash_additional_change_set(additional_change_set, &MockChangeSetChecker)
    );

    let mut descriptor = "r";
    assert_eq!(
        change_set.resource_write_set(),
        &expected_write_set!(descriptor)
    );
    descriptor = "m";
    assert_eq!(
        change_set.module_write_set(),
        &expected_write_set!(descriptor)
    );

    let expected_aggregator_write_set = BTreeMap::from([
        mock_create("18a", 136),
        mock_modify("19a", 138),
        mock_modify("22a", 122),
        mock_delete("23a"),
    ]);
    let expected_aggregator_delta_set = BTreeMap::from([
        mock_add("15a", 15),
        mock_add("16a", 116),
        mock_add("17a", 134),
    ]);
    assert_eq!(
        change_set.aggregator_write_set(),
        &expected_aggregator_write_set
    );
    assert_eq!(
        change_set.aggregator_delta_set(),
        &expected_aggregator_delta_set
    );
}

macro_rules! assert_invariant_violation {
    ($w1:ident, $w2:ident) => {
        let check = |res: anyhow::Result<(), VMStatus>| {
            assert_matches!(
                res,
                Err(VMStatus::Error {
                    status_code: StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                    sub_status: None,
                    message: Some(_),
                })
            );
        };

        let mut cs1 = build_change_set($w1.clone(), vec![], vec![], vec![]);
        let cs2 = build_change_set($w2.clone(), vec![], vec![], vec![]);
        let res = cs1.squash_additional_change_set(cs2, &MockChangeSetChecker);
        check(res);
        let mut cs1 = build_change_set(vec![], $w1.clone(), vec![], vec![]);
        let cs2 = build_change_set(vec![], $w2.clone(), vec![], vec![]);
        let res = cs1.squash_additional_change_set(cs2, &MockChangeSetChecker);
        check(res);
        let mut cs1 = build_change_set(vec![], vec![], $w1.clone(), vec![]);
        let cs2 = build_change_set(vec![], vec![], $w2.clone(), vec![]);
        let res = cs1.squash_additional_change_set(cs2, &MockChangeSetChecker);
        check(res);
    };
}

#[test]
fn test_unsuccessful_squash_create_create() {
    // create 6 + create 106 throws an error
    let write_set_1 = vec![mock_create("6", 6)];
    let write_set_2 = vec![mock_create("6", 106)];
    assert_invariant_violation!(write_set_1, write_set_2);
}

#[test]
fn test_unsuccessful_squash_modify_create() {
    // modify 9 + create 109 throws an error
    let write_set_1 = vec![mock_modify("9", 9)];
    let write_set_2 = vec![mock_create("9", 109)];
    assert_invariant_violation!(write_set_1, write_set_2);
}

#[test]
fn test_unsuccessful_squash_delete_modify() {
    // delete + modify 113 throws an error
    let write_set_1 = vec![mock_delete("13")];
    let write_set_2 = vec![mock_modify("13", 113)];
    assert_invariant_violation!(write_set_1, write_set_2);
}

#[test]
fn test_unsuccessful_squash_delete_delete() {
    // delete + delete throws an error
    let write_set_1 = vec![mock_delete("14")];
    let write_set_2 = vec![mock_delete("14")];
    assert_invariant_violation!(write_set_1, write_set_2);
}

#[test]
fn test_unsuccessful_squash_delete_delta() {
    // delete + +120 throws an error
    let aggregator_write_set_1 = vec![mock_delete("20")];
    let aggregator_delta_set_2 = vec![mock_add("20", 120)];

    let mut change_set = build_change_set(vec![], vec![], aggregator_write_set_1, vec![]);
    let additional_change_set = build_change_set(vec![], vec![], vec![], aggregator_delta_set_2);
    let res = change_set.squash_additional_change_set(additional_change_set, &MockChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error {
            status_code: StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            sub_status: None,
            message: Some(_),
        })
    );
}

#[test]
fn test_unsuccessful_squash_delta_create() {
    // +21 + create 122 throws an error
    let aggregator_delta_set_1 = vec![mock_add("21", 21)];
    let aggregator_write_set_2 = vec![mock_create("21", 121)];

    let mut change_set = build_change_set(vec![], vec![], vec![], aggregator_delta_set_1);
    let additional_change_set = build_change_set(vec![], vec![], aggregator_write_set_2, vec![]);
    let res = change_set.squash_additional_change_set(additional_change_set, &MockChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error {
            status_code: StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            sub_status: None,
            message: Some(_),
        })
    );
}

#[test]
fn test_roundtrip_to_storage_change_set() {
    let test_struct_tag = StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").into(),
        name: ident_str!("Foo").into(),
        type_params: vec![],
    };
    let test_module_id = ModuleId::new(AccountAddress::ONE, ident_str!("bar").into());

    let resource_key = StateKey::access_path(
        AccessPath::resource_access_path(AccountAddress::ONE, test_struct_tag).unwrap(),
    );
    let module_key = StateKey::access_path(AccessPath::code_access_path(test_module_id));
    let write_set = WriteSetMut::new(vec![
        (resource_key, WriteOp::Deletion),
        (module_key, WriteOp::Deletion),
    ])
    .freeze()
    .unwrap();

    let storage_change_set_before = StorageChangeSet::new(write_set, vec![]);
    let change_set = assert_ok!(VMChangeSet::try_from_storage_change_set(
        storage_change_set_before.clone(),
        &MockChangeSetChecker
    ));
    let storage_change_set_after = assert_ok!(change_set.try_into_storage_change_set());
    assert_eq!(storage_change_set_before, storage_change_set_after)
}

#[test]
fn test_failed_conversion_to_change_set() {
    let resource_write_set = vec![mock_delete("a")];
    let aggregator_delta_set = vec![mock_add("b", 100)];
    let change_set = build_change_set(resource_write_set, vec![], vec![], aggregator_delta_set);

    // Unchecked conversion ignores deltas.
    let storage_change_set = change_set.clone().into_storage_change_set_unchecked();
    assert_eq!(storage_change_set.write_set().clone().into_mut().len(), 1);

    let vm_status = change_set.try_into_storage_change_set();
    assert_matches!(
        vm_status,
        Err(VMStatus::Error {
            status_code: StatusCode::DATA_FORMAT_ERROR,
            sub_status: None,
            message: Some(_),
        })
    );
}

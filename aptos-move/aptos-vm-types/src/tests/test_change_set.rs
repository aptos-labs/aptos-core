// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    change_set::VMChangeSet,
    tests::utils::{
        build_change_set, contains_delta_op, contains_write_op, create, delete, get_delta_op,
        get_write_op, key, modify, NoOpChangeSetChecker,
    },
};
use aptos_aggregator::delta_change_set::{delta_add, DeltaChangeSet};
use aptos_types::write_set::WriteSetMut;
use claims::{assert_matches, assert_ok};
use move_core_types::vm_status::{StatusCode, VMStatus};

macro_rules! add {
    ($v:expr) => {
        // Limit doesn't matter here, so set it to be relatively high.
        delta_add($v, 100000)
    };
}

/// Returns two change sets according tow specification:
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
fn build_change_sets_for_test() -> (VMChangeSet, VMChangeSet) {
    // Create write sets and delta change sets.
    let mut write_set_1 = WriteSetMut::default();
    let mut write_set_2 = WriteSetMut::default();
    let mut delta_change_set_1 = DeltaChangeSet::empty();
    let mut delta_change_set_2 = DeltaChangeSet::empty();

    // Populate sets according to the spec. Skip keys which lead to
    // errors because we test them separately.
    write_set_1.insert((key(0), create(0)));
    write_set_1.insert((key(1), modify(1)));
    write_set_1.insert((key(2), delete()));
    write_set_2.insert((key(3), create(103)));
    write_set_2.insert((key(4), modify(104)));
    write_set_2.insert((key(5), delete()));

    write_set_1.insert((key(7), create(7)));
    write_set_2.insert((key(7), modify(107)));
    write_set_1.insert((key(8), create(8)));
    write_set_2.insert((key(8), delete()));

    write_set_1.insert((key(10), modify(10)));
    write_set_2.insert((key(10), modify(110)));
    write_set_1.insert((key(11), modify(111)));
    write_set_2.insert((key(11), delete()));
    write_set_1.insert((key(12), delete()));
    write_set_2.insert((key(12), create(112)));

    delta_change_set_1.insert((key(15), add!(15)));
    delta_change_set_2.insert((key(16), add!(116)));
    delta_change_set_1.insert((key(17), add!(17)));
    delta_change_set_2.insert((key(17), add!(117)));
    write_set_1.insert((key(18), create(18)));
    delta_change_set_2.insert((key(18), add!(118)));
    write_set_1.insert((key(19), modify(19)));
    delta_change_set_2.insert((key(19), add!(119)));

    delta_change_set_1.insert((key(22), add!(22)));
    write_set_2.insert((key(22), modify(122)));
    delta_change_set_1.insert((key(23), add!(23)));
    write_set_2.insert((key(23), delete()));

    (
        build_change_set(write_set_1, delta_change_set_1),
        build_change_set(write_set_2, delta_change_set_2),
    )
}

#[test]
fn test_successful_squash() {
    let (change_set_1, change_set_2) = build_change_sets_for_test();

    // Check squash is indeed successful.
    let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
    let change_set = assert_ok!(res);

    // create 0 + ___ = create 0
    assert_eq!(get_write_op(&change_set, 0), create(0));
    assert!(!contains_delta_op(&change_set, 0));

    // modify 1 + ___ = modify 1
    assert_eq!(get_write_op(&change_set, 1), modify(1));
    assert!(!contains_delta_op(&change_set, 1));

    // delete + ___ = delete
    assert_eq!(get_write_op(&change_set, 2), delete());
    assert!(!contains_delta_op(&change_set, 2));

    // ___ + create 103 = create 103
    assert_eq!(get_write_op(&change_set, 3), create(103));
    assert!(!contains_delta_op(&change_set, 3));

    // ___ + modify 103 = modify 103
    assert_eq!(get_write_op(&change_set, 4), modify(104));
    assert!(!contains_delta_op(&change_set, 4));

    // ___ + delete = delete
    assert_eq!(get_write_op(&change_set, 5), delete());
    assert!(!contains_delta_op(&change_set, 5));

    // create 7 + modify 107 = create 107
    assert_eq!(get_write_op(&change_set, 7), create(107));
    assert!(!contains_delta_op(&change_set, 7));

    // create 8 + delete = ___
    assert!(!contains_write_op(&change_set, 8));
    assert!(!contains_delta_op(&change_set, 8));

    // modify 10 + modify 110 = modify 110
    assert_eq!(get_write_op(&change_set, 10), modify(110));
    assert!(!contains_delta_op(&change_set, 10));

    // modify 10 + delete = delete
    assert_eq!(get_write_op(&change_set, 11), delete());
    assert!(!contains_delta_op(&change_set, 11));

    // delete + create 112 = create 112
    assert_eq!(get_write_op(&change_set, 12), modify(112));
    assert!(!contains_delta_op(&change_set, 12));

    // +15 + ___ = +15
    assert!(!contains_write_op(&change_set, 15));
    assert_eq!(get_delta_op(&change_set, 15), add!(15));

    // ___ + +116 = +116
    assert!(!contains_write_op(&change_set, 16));
    assert_eq!(get_delta_op(&change_set, 16), add!(116));

    // +17 + +117 = +134
    assert!(!contains_write_op(&change_set, 17));
    assert_eq!(get_delta_op(&change_set, 17), add!(134));

    // create 18 + +118 = create 136
    assert_eq!(get_write_op(&change_set, 18), create(136));
    assert!(!contains_delta_op(&change_set, 18));

    // modify 19 + +119 = modify 138
    assert_eq!(get_write_op(&change_set, 19), modify(138));
    assert!(!contains_delta_op(&change_set, 19));

    // +22 + modify 122 = modify 122
    assert_eq!(get_write_op(&change_set, 22), modify(122));
    assert!(!contains_delta_op(&change_set, 22));

    // +23 + delete = delete
    assert_eq!(get_write_op(&change_set, 23), delete());
    assert!(!contains_delta_op(&change_set, 23));
}

#[test]
fn test_unsuccessful_squash_1() {
    let mut write_set_1 = WriteSetMut::default();
    let mut write_set_2 = WriteSetMut::default();

    // create 6 + create 106 throws an error
    write_set_1.insert((key(6), create(6)));
    write_set_2.insert((key(6), create(106)));

    let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
    let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
    let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            Some(_),
        ))
    );
}

#[test]
fn test_unsuccessful_squash_modify_create() {
    let mut write_set_1 = WriteSetMut::default();
    let mut write_set_2 = WriteSetMut::default();

    // modify 9 + create 109 throws an error
    write_set_1.insert((key(9), modify(9)));
    write_set_2.insert((key(9), create(109)));

    let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
    let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
    let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            Some(_),
        ))
    );
}

#[test]
fn test_unsuccessful_squash_delete_modify() {
    let mut write_set_1 = WriteSetMut::default();
    let mut write_set_2 = WriteSetMut::default();

    // delete + modify 113 throws an error
    write_set_1.insert((key(13), delete()));
    write_set_2.insert((key(13), modify(113)));

    let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
    let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
    let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            Some(_),
        ))
    );
}

#[test]
fn test_unsuccessful_squash_delete_delete() {
    let mut write_set_1 = WriteSetMut::default();
    let mut write_set_2 = WriteSetMut::default();

    // delete + delete throws an error
    write_set_1.insert((key(14), delete()));
    write_set_2.insert((key(14), delete()));

    let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
    let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
    let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            Some(_),
        ))
    );
}

#[test]
fn test_unsuccessful_squash_delete_delta() {
    let mut write_set_1 = WriteSetMut::default();
    let mut delta_change_set_2 = DeltaChangeSet::empty();

    // delete + +120 throws an error
    write_set_1.insert((key(20), delete()));
    delta_change_set_2.insert((key(20), add!(120)));

    let change_set_1 = build_change_set(write_set_1, DeltaChangeSet::empty());
    let change_set_2 = build_change_set(WriteSetMut::default(), delta_change_set_2);
    let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            Some(_),
        ))
    );
}

#[test]
fn test_unsuccessful_squash_delta_create() {
    let mut write_set_2 = WriteSetMut::default();
    let mut delta_change_set_1 = DeltaChangeSet::empty();

    // +21 + create 122 throws an error
    delta_change_set_1.insert((key(21), add!(21)));
    write_set_2.insert((key(21), create(121)));

    let change_set_1 = build_change_set(WriteSetMut::default(), delta_change_set_1);
    let change_set_2 = build_change_set(write_set_2, DeltaChangeSet::empty());
    let res = change_set_1.squash(change_set_2, &NoOpChangeSetChecker);
    assert_matches!(
        res,
        Err(VMStatus::Error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            Some(_),
        ))
    );
}

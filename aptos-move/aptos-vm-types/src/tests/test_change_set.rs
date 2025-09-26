// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::utils::{mock_tag_0, VMChangeSetBuilder};
use crate::{
    abstract_write_op::{AbstractResourceWriteOp, GroupWrite},
    change_set::{
        create_vm_change_set_with_module_write_set_when_delayed_field_optimization_disabled,
        VMChangeSet,
    },
    module_write_set::ModuleWriteSet,
    resolver::ResourceGroupSize,
    tests::utils::{
        as_bytes, as_state_key, mock_add, mock_create, mock_create_with_layout, mock_delete,
        mock_delete_with_layout, mock_modify, mock_modify_with_layout, mock_tag_1, raw_metadata,
        ExpandedVMChangeSetBuilder,
    },
};
use aptos_aggregator::{
    bounded_math::SignedU128,
    delayed_change::{DelayedApplyChange, DelayedChange},
    delta_change_set::DeltaWithMax,
};
use aptos_types::{
    delayed_fields::SnapshotToStringFormula,
    error::PanicError,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    transaction::ChangeSet as StorageChangeSet,
    write_set::{WriteOp, WriteSetMut},
};
use bytes::Bytes;
use claims::{assert_err, assert_matches, assert_ok, assert_some_eq};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{ModuleId, StructTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::collections::BTreeMap;
use triomphe::Arc as TriompheArc;

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

macro_rules! resource_write_set_1 {
    ($d:ident) => {
        vec![
            mock_create_with_layout(format!("0{}", $d), 0, None),
            mock_modify_with_layout(format!("1{}", $d), 1, None),
            mock_delete_with_layout(format!("2{}", $d)),
            mock_create_with_layout(format!("7{}", $d), 7, None),
            mock_create_with_layout(format!("8{}", $d), 8, None),
            mock_modify_with_layout(format!("10{}", $d), 10, None),
            mock_modify_with_layout(format!("11{}", $d), 11, None),
            mock_delete_with_layout(format!("12{}", $d)),
        ]
    };
}

macro_rules! resource_write_set_2 {
    ($d:ident) => {
        vec![
            mock_create_with_layout(format!("3{}", $d), 103, None),
            mock_modify_with_layout(format!("4{}", $d), 104, None),
            mock_delete_with_layout(format!("5{}", $d)),
            mock_modify_with_layout(format!("7{}", $d), 107, None),
            mock_delete_with_layout(format!("8{}", $d)),
            mock_modify_with_layout(format!("10{}", $d), 110, None),
            mock_delete_with_layout(format!("11{}", $d)),
            mock_create_with_layout(format!("12{}", $d), 112, None),
        ]
    };
}

macro_rules! expected_resource_write_set {
    ($d:ident) => {
        BTreeMap::from([
            mock_create_with_layout(format!("0{}", $d), 0, None),
            mock_modify_with_layout(format!("1{}", $d), 1, None),
            mock_delete_with_layout(format!("2{}", $d)),
            mock_create_with_layout(format!("3{}", $d), 103, None),
            mock_modify_with_layout(format!("4{}", $d), 104, None),
            mock_delete_with_layout(format!("5{}", $d)),
            mock_create_with_layout(format!("7{}", $d), 107, None),
            mock_modify_with_layout(format!("10{}", $d), 110, None),
            mock_delete_with_layout(format!("11{}", $d)),
            mock_modify_with_layout(format!("12{}", $d), 112, None),
        ])
    };
}

// Populate sets according to the spec. Skip keys which lead to
// errors because we test them separately.
fn build_change_sets_for_test() -> (VMChangeSet, VMChangeSet) {
    let mut descriptor = "r";
    let resource_write_set_1 = resource_write_set_1!(descriptor);
    let aggregator_write_set_1 = vec![mock_create("18a", 18), mock_modify("19a", 19)];
    let aggregator_delta_set_1 = vec![
        mock_add("15a", 15),
        mock_add("17a", 17),
        mock_add("22a", 22),
        mock_add("23a", 23),
    ];
    let change_set_1 = VMChangeSetBuilder::new()
        .with_resource_write_set(resource_write_set_1)
        .with_aggregator_v1_write_set(aggregator_write_set_1)
        .with_aggregator_v1_delta_set(aggregator_delta_set_1)
        .build();

    descriptor = "r";
    let resource_write_set_2 = resource_write_set_2!(descriptor);
    let aggregator_write_set_2 = vec![mock_modify("22a", 122), mock_delete("23a")];
    let aggregator_delta_set_2 = vec![
        mock_add("16a", 116),
        mock_add("17a", 117),
        mock_add("18a", 118),
        mock_add("19a", 119),
    ];
    let change_set_2 = VMChangeSetBuilder::new()
        .with_resource_write_set(resource_write_set_2)
        .with_aggregator_v1_write_set(aggregator_write_set_2)
        .with_aggregator_v1_delta_set(aggregator_delta_set_2)
        .build();

    (change_set_1, change_set_2)
}

#[test]
fn test_successful_squash() {
    let (mut change_set, additional_change_set) = build_change_sets_for_test();
    assert_ok!(change_set.squash_additional_change_set(additional_change_set,));

    let descriptor = "r";
    assert_eq!(
        change_set.resource_write_set(),
        &expected_resource_write_set!(descriptor)
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
        change_set.aggregator_v1_write_set(),
        &expected_aggregator_write_set
    );
    assert_eq!(
        change_set.aggregator_v1_delta_set(),
        &expected_aggregator_delta_set
    );
}

macro_rules! assert_invariant_violation {
    ($w1:ident, $w2:ident, $w3:ident, $w4:ident) => {
        let check = |res: PartialVMResult<()>| {
            let err = assert_err!(res);

            // TODO[agg_v2](test): Uniformize errors for write op squashing.
            assert!(
                err.major_status() == StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
                    || err.major_status()
                        == StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR
            );
        };

        let mut cs1 = VMChangeSetBuilder::new()
            .with_resource_write_set($w1.clone())
            .build();
        let cs2 = VMChangeSetBuilder::new()
            .with_resource_write_set($w2.clone())
            .build();
        let res = cs1.squash_additional_change_set(cs2);
        check(res);
        let mut cs1 = VMChangeSetBuilder::new()
            .with_aggregator_v1_write_set($w3.clone())
            .build();
        let cs2 = VMChangeSetBuilder::new()
            .with_aggregator_v1_write_set($w4.clone())
            .build();
        let res = cs1.squash_additional_change_set(cs2);
        check(res);
    };
}

#[test]
fn test_unsuccessful_squash_create_create() {
    // create 6 + create 106 throws an error
    let write_set_1 = vec![mock_create_with_layout("6", 6, None)];
    let write_set_2 = vec![mock_create_with_layout("6", 106, None)];
    let write_set_3 = vec![mock_create("6", 6)];
    let write_set_4 = vec![mock_create("6", 106)];
    assert_invariant_violation!(write_set_1, write_set_2, write_set_3, write_set_4);
}

#[test]
fn test_unsuccessful_squash_modify_create() {
    // modify 9 + create 109 throws an error
    let write_set_1 = vec![mock_modify_with_layout("9", 9, None)];
    let write_set_2 = vec![mock_create_with_layout("9", 109, None)];
    let write_set_3 = vec![mock_modify("9", 9)];
    let write_set_4 = vec![mock_create("9", 109)];
    assert_invariant_violation!(write_set_1, write_set_2, write_set_3, write_set_4);
}

#[test]
fn test_unsuccessful_squash_delete_modify() {
    // delete + modify 113 throws an error
    let write_set_1 = vec![mock_delete_with_layout("13")];
    let write_set_2 = vec![mock_modify_with_layout("13", 113, None)];
    let write_set_3 = vec![mock_delete("13")];
    let write_set_4 = vec![mock_modify("13", 113)];
    assert_invariant_violation!(write_set_1, write_set_2, write_set_3, write_set_4);
}

#[test]
fn test_unsuccessful_squash_delete_delete() {
    // delete + delete throws an error
    let write_set_1 = vec![mock_delete_with_layout("14")];
    let write_set_2 = vec![mock_delete_with_layout("14")];
    let write_set_3 = vec![mock_delete("14")];
    let write_set_4 = vec![mock_delete("14")];
    assert_invariant_violation!(write_set_1, write_set_2, write_set_3, write_set_4);
}

#[test]
fn test_unsuccessful_squash_delete_delta() {
    // delete + +120 throws an error
    let aggregator_write_set_1 = vec![mock_delete("20")];
    let aggregator_delta_set_2 = vec![mock_add("20", 120)];

    let mut change_set = VMChangeSetBuilder::new()
        .with_aggregator_v1_write_set(aggregator_write_set_1)
        .build();
    let additional_change_set = VMChangeSetBuilder::new()
        .with_aggregator_v1_delta_set(aggregator_delta_set_2)
        .build();
    let res = change_set.squash_additional_change_set(additional_change_set);
    let err = assert_err!(res);
    assert_eq!(
        err.major_status(),
        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
    );
}

#[test]
fn test_unsuccessful_squash_delta_create() {
    // +21 + create 122 throws an error
    let aggregator_delta_set_1 = vec![mock_add("21", 21)];
    let aggregator_write_set_2 = vec![mock_create("21", 121)];

    let mut change_set = VMChangeSetBuilder::new()
        .with_aggregator_v1_delta_set(aggregator_delta_set_1)
        .build();
    let additional_change_set = VMChangeSetBuilder::new()
        .with_aggregator_v1_write_set(aggregator_write_set_2)
        .build();
    let res = change_set.squash_additional_change_set(additional_change_set);
    let err = assert_err!(res);
    assert_eq!(
        err.major_status(),
        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
    );
}

#[test]
fn test_roundtrip_to_storage_change_set() {
    let test_struct_tag = StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("foo").into(),
        name: ident_str!("Foo").into(),
        type_args: vec![],
    };
    let test_module_id = ModuleId::new(AccountAddress::ONE, ident_str!("bar").into());

    let resource_key = StateKey::resource(&AccountAddress::ONE, &test_struct_tag).unwrap();
    let module_key = StateKey::module_id(&test_module_id);
    let write_set = WriteSetMut::new(vec![
        (resource_key, WriteOp::legacy_deletion()),
        (module_key, WriteOp::legacy_deletion()),
    ])
    .freeze()
    .unwrap();

    let storage_change_set_before = StorageChangeSet::new(write_set, vec![]);
    let (change_set, module_write_set) =
        create_vm_change_set_with_module_write_set_when_delayed_field_optimization_disabled(
            storage_change_set_before.clone(),
        );

    let storage_change_set_after =
        assert_ok!(change_set.try_combine_into_storage_change_set(module_write_set));
    assert_eq!(storage_change_set_before, storage_change_set_after)
}

#[test]
fn test_failed_conversion_to_change_set() {
    let resource_write_set = vec![mock_delete_with_layout("a")];
    let aggregator_delta_set = vec![mock_add("b", 100)];
    let change_set = VMChangeSetBuilder::new()
        .with_resource_write_set(resource_write_set)
        .with_aggregator_v1_delta_set(aggregator_delta_set)
        .build();

    // Unchecked conversion ignores deltas.
    let vm_status = change_set.try_combine_into_storage_change_set(ModuleWriteSet::empty());
    assert_matches!(vm_status, Err(PanicError::CodeInvariantError(_)));
}

#[test]
fn test_conversion_to_change_set_fails() {
    let resource_write_set = vec![mock_delete_with_layout("a")];
    let aggregator_delta_set = vec![mock_add("b", 100)];
    let change_set = VMChangeSetBuilder::new()
        .with_resource_write_set(resource_write_set)
        .with_aggregator_v1_delta_set(aggregator_delta_set)
        .build();

    assert_err!(change_set
        .clone()
        .try_combine_into_storage_change_set(ModuleWriteSet::empty()));
}

#[test]
fn test_aggregator_v2_snapshots_and_derived() {
    use DelayedApplyChange::*;
    use DelayedChange::*;

    let agg_changes_1 = vec![(
        DelayedFieldID::new_for_test_for_u64(1),
        Apply(AggregatorDelta {
            delta: DeltaWithMax::new(SignedU128::Positive(3), 100),
        }),
    )];
    let mut change_set_1 = VMChangeSetBuilder::new()
        .with_delayed_field_change_set(agg_changes_1)
        .build();

    let agg_changes_2 = vec![
        (
            DelayedFieldID::new_for_test_for_u64(1),
            Apply(AggregatorDelta {
                delta: DeltaWithMax::new(SignedU128::Positive(5), 100),
            }),
        ),
        (
            DelayedFieldID::new_for_test_for_u64(2),
            Apply(SnapshotDelta {
                base_aggregator: DelayedFieldID::new_for_test_for_u64(1),
                delta: DeltaWithMax::new(SignedU128::Positive(2), 100),
            }),
        ),
        (
            DelayedFieldID::new_for_test_for_u64(3),
            Apply(SnapshotDerived {
                base_snapshot: DelayedFieldID::new_for_test_for_u64(2),
                formula: SnapshotToStringFormula::Concat {
                    prefix: "p".as_bytes().to_vec(),
                    suffix: "s".as_bytes().to_vec(),
                },
            }),
        ),
    ];
    let change_set_2 = VMChangeSetBuilder::new()
        .with_delayed_field_change_set(agg_changes_2)
        .build();

    assert_ok!(change_set_1.squash_additional_change_set(change_set_2,));

    let output_map = change_set_1.delayed_field_change_set();
    assert_eq!(output_map.len(), 3);
    assert_some_eq!(
        output_map.get(&DelayedFieldID::new_for_test_for_u64(1)),
        &Apply(AggregatorDelta {
            delta: DeltaWithMax::new(SignedU128::Positive(8), 100)
        })
    );
    assert_some_eq!(
        output_map.get(&DelayedFieldID::new_for_test_for_u64(2)),
        &Apply(SnapshotDelta {
            base_aggregator: DelayedFieldID::new_for_test_for_u64(1),
            delta: DeltaWithMax::new(SignedU128::Positive(5), 100)
        })
    );
    assert_some_eq!(
        output_map.get(&DelayedFieldID::new_for_test_for_u64(3)),
        &Apply(SnapshotDerived {
            base_snapshot: DelayedFieldID::new_for_test_for_u64(2),
            formula: SnapshotToStringFormula::Concat {
                prefix: "p".as_bytes().to_vec(),
                suffix: "s".as_bytes().to_vec()
            },
        })
    );
}

#[test]
fn test_resource_groups_squashing() {
    let modification_metadata = WriteOp::modification(Bytes::new(), raw_metadata(2000));

    macro_rules! as_create_op {
        ($val:expr) => {
            (WriteOp::legacy_creation(as_bytes!($val).into()), None)
        };
    }
    macro_rules! as_modify_op {
        ($val:expr) => {
            (WriteOp::legacy_modification(as_bytes!($val).into()), None)
        };
    }

    let create_tag_0_op = (mock_tag_0(), as_create_op!(5));
    let single_tag_group_size = ResourceGroupSize::Combined {
        num_tagged_resources: 1,
        all_tagged_resources_size: 100,
    };
    let two_tag_group_size = ResourceGroupSize::Combined {
        num_tagged_resources: 2,
        all_tagged_resources_size: 200,
    };
    let create_group_write_0 = GroupWrite::new(
        modification_metadata.clone(),
        BTreeMap::from([create_tag_0_op.clone()]),
        single_tag_group_size,
        0,
    );
    let create_tag_0 = ExpandedVMChangeSetBuilder::new()
        .with_resource_group_write_set(vec![(as_state_key!("1"), create_group_write_0.clone())])
        .build();

    let modify_group_write_0 = GroupWrite::new(
        modification_metadata.clone(),
        BTreeMap::from([(mock_tag_0(), as_modify_op!(7))]),
        single_tag_group_size,
        single_tag_group_size.get(),
    );
    let modify_tag_0 = ExpandedVMChangeSetBuilder::new()
        .with_resource_group_write_set(vec![(as_state_key!("1"), modify_group_write_0.clone())])
        .build();

    let create_tag_1_op = (mock_tag_1(), as_create_op!(15));
    let create_group_write_1 = GroupWrite::new(
        modification_metadata.clone(),
        BTreeMap::from([create_tag_1_op.clone()]),
        two_tag_group_size,
        single_tag_group_size.get(),
    );
    let create_tag_1 = ExpandedVMChangeSetBuilder::new()
        .with_resource_group_write_set(vec![(as_state_key!("1"), create_group_write_1.clone())])
        .build();

    let modify_tag_1_op = (mock_tag_1(), as_modify_op!(17));
    let modify_group_write_1 = GroupWrite::new(
        modification_metadata.clone(),
        BTreeMap::from([modify_tag_1_op.clone()]),
        two_tag_group_size,
        two_tag_group_size.get(),
    );
    let modify_tag_1 = ExpandedVMChangeSetBuilder::new()
        .with_resource_group_write_set(vec![(as_state_key!("1"), modify_group_write_1.clone())])
        .build();

    {
        let mut change_set = create_tag_0.clone();
        assert_ok!(change_set.squash_additional_change_set(modify_tag_0.clone(),));
        assert_eq!(change_set.resource_write_set().len(), 1);
        // create(x)+modify(y) becomes create(y)
        assert_some_eq!(
            change_set.resource_write_set().get(&as_state_key!("1")),
            &AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
                modification_metadata.clone(),
                BTreeMap::from([(mock_tag_0(), as_create_op!(7))]),
                single_tag_group_size,
                0,
            ))
        );
    }

    {
        let mut change_set = create_tag_0.clone();
        assert_ok!(change_set.squash_additional_change_set(create_tag_1.clone(),));
        assert_eq!(change_set.resource_write_set().len(), 1);
        assert_some_eq!(
            change_set.resource_write_set().get(&as_state_key!("1")),
            &AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
                modification_metadata.clone(),
                BTreeMap::from([create_tag_0_op.clone(), create_tag_1_op.clone()]),
                two_tag_group_size,
                0,
            ))
        );

        assert_ok!(change_set.squash_additional_change_set(modify_tag_1.clone(),));
        assert_eq!(change_set.resource_write_set().len(), 1);
        // create(x)+modify(y) becomes create(y)
        assert_some_eq!(
            change_set.resource_write_set().get(&as_state_key!("1")),
            &AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
                modification_metadata.clone(),
                BTreeMap::from([create_tag_0_op.clone(), (mock_tag_1(), as_create_op!(17))]),
                two_tag_group_size,
                0,
            ))
        );
    }

    {
        let mut change_set = create_tag_0.clone();
        assert_ok!(change_set.squash_additional_change_set(modify_tag_1.clone(),));
        assert_eq!(change_set.resource_write_set().len(), 1);
        assert_some_eq!(
            change_set.resource_write_set().get(&as_state_key!("1")),
            &AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
                modification_metadata.clone(),
                BTreeMap::from([create_tag_0_op.clone(), modify_tag_1_op.clone()]),
                two_tag_group_size,
                0,
            ))
        );
    }

    {
        // read cannot modify size
        let mut change_set = create_tag_0.clone();
        assert_err!(change_set.squash_additional_change_set(
            ExpandedVMChangeSetBuilder::new()
                .with_group_reads_needing_delayed_field_exchange(vec![(
                    as_state_key!("1"),
                    (modification_metadata.metadata().clone(), 400)
                )])
                .build(),
        ));
    }
}

#[test]
fn test_write_and_read_discrepancy_caught() {
    assert_err!(ExpandedVMChangeSetBuilder::new()
        .with_resource_write_set(vec![(
            as_state_key!("1"),
            (WriteOp::legacy_modification(as_bytes!(1).into()), None),
        )])
        .with_reads_needing_delayed_field_exchange(vec![(
            as_state_key!("1"),
            (
                StateValueMetadata::none(),
                10,
                TriompheArc::new(MoveTypeLayout::U64)
            )
        )])
        .try_build());

    let metadata_op = WriteOp::modification(Bytes::new(), raw_metadata(1000));
    let group_size = ResourceGroupSize::Combined {
        num_tagged_resources: 1,
        all_tagged_resources_size: 14,
    };

    assert_err!(ExpandedVMChangeSetBuilder::new()
        .with_resource_group_write_set(vec![(
            as_state_key!("1"),
            GroupWrite::new(
                metadata_op.clone(),
                BTreeMap::new(),
                group_size,
                group_size.get()
            )
        )])
        .with_group_reads_needing_delayed_field_exchange(vec![(
            as_state_key!("1"),
            (metadata_op.metadata().clone(), group_size.get())
        )])
        .try_build());
}

// TODO[agg_v2](cleanup) combine utilities with above utilities, and see if tests need cleanup.
// below are moved from change_set.rs, to consolidate.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::{mock_tag_0, mock_tag_1, mock_tag_2, raw_metadata};
    use bytes::Bytes;
    use claims::{assert_err, assert_ok, assert_some_eq};
    use move_core_types::language_storage::StructTag;
    use test_case::test_case;

    const CREATION: u8 = 0;
    const MODIFICATION: u8 = 1;
    const DELETION: u8 = 2;

    pub(crate) fn write_op_with_metadata(type_idx: u8, v: u128) -> WriteOp {
        match type_idx {
            CREATION => WriteOp::creation(vec![].into(), raw_metadata(v as u64)),
            MODIFICATION => WriteOp::modification(vec![].into(), raw_metadata(v as u64)),
            DELETION => WriteOp::deletion(raw_metadata(v as u64)),
            _ => unreachable!("Wrong type index for test"),
        }
    }

    fn group_write(
        metadata_op: WriteOp,
        inner_ops: Vec<(StructTag, (WriteOp, Option<TriompheArc<MoveTypeLayout>>))>,
        num_tagged_resources: usize,
        all_tagged_resources_size: u64,
    ) -> AbstractResourceWriteOp {
        let group_size = ResourceGroupSize::Combined {
            num_tagged_resources,
            all_tagged_resources_size,
        };
        AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
            metadata_op,
            inner_ops.into_iter().collect(),
            group_size,
            group_size.get(), // prev_group_size
        ))
    }

    fn extract_group_op(write_op: &AbstractResourceWriteOp) -> &GroupWrite {
        if let AbstractResourceWriteOp::WriteResourceGroup(write_op) = write_op {
            write_op
        } else {
            panic!("Expected WriteResourceGroup, got {:?}", write_op)
        }
    }

    macro_rules! assert_group_write_size {
        ($op:expr, $s:expr, $exp:expr) => {{
            let group_write = GroupWrite::new($op, BTreeMap::new(), $s, $s.get());
            assert_eq!(group_write.maybe_group_op_size(), $exp);
        }};
    }

    #[test]
    fn test_group_write_size() {
        // Deletions should lead to size 0.
        assert_group_write_size!(
            WriteOp::legacy_deletion(),
            ResourceGroupSize::zero_combined(),
            None
        );
        assert_group_write_size!(
            WriteOp::deletion(raw_metadata(10)),
            ResourceGroupSize::zero_combined(),
            None
        );

        let sizes = [
            ResourceGroupSize::Combined {
                num_tagged_resources: 1,
                all_tagged_resources_size: 20,
            },
            ResourceGroupSize::Combined {
                num_tagged_resources: 1,
                all_tagged_resources_size: 100,
            },
            ResourceGroupSize::Combined {
                num_tagged_resources: 1,
                all_tagged_resources_size: 45279432,
            },
            ResourceGroupSize::Combined {
                num_tagged_resources: 1,
                all_tagged_resources_size: 5,
            },
            ResourceGroupSize::Combined {
                num_tagged_resources: 1024,
                all_tagged_resources_size: 45279432,
            },
        ];
        assert_group_write_size!(
            WriteOp::legacy_creation(Bytes::new()),
            sizes[0],
            Some(sizes[0])
        );
        assert_group_write_size!(
            WriteOp::creation(Bytes::new(), raw_metadata(20)),
            sizes[1],
            Some(sizes[1])
        );
        assert_group_write_size!(
            WriteOp::legacy_modification(Bytes::new()),
            sizes[2],
            Some(sizes[2])
        );
        assert_group_write_size!(
            WriteOp::modification(Bytes::new(), raw_metadata(30)),
            sizes[3],
            Some(sizes[3])
        );
    }

    #[test]
    fn test_squash_groups_one_empty() {
        let key_1 = StateKey::raw(&[1]);
        let key_2 = StateKey::raw(&[2]);

        let mut base_update = BTreeMap::new();
        base_update.insert(
            key_1.clone(),
            group_write(write_op_with_metadata(CREATION, 100), vec![], 0, 0),
        );
        let mut additional_update = BTreeMap::new();
        additional_update.insert(
            key_2.clone(),
            group_write(write_op_with_metadata(CREATION, 200), vec![], 0, 0),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 2);
        assert_eq!(
            extract_group_op(base_update.get(&key_1).unwrap())
                .metadata_op
                .metadata(),
            &raw_metadata(100)
        );
        assert_eq!(
            extract_group_op(base_update.get(&key_2).unwrap())
                .metadata_op
                .metadata(),
            &raw_metadata(200)
        );
    }

    #[test_case(0, 1)] // create, modify
    #[test_case(1, 1)] // modify, modify
    #[test_case(1, 2)] // modify, delete
    #[test_case(2, 0)] // delete, create
    fn test_squash_groups_mergeable_metadata(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(&[0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(
            key.clone(),
            group_write(write_op_with_metadata(base_type_idx, 100), vec![], 0, 0),
        );
        additional_update.insert(
            key.clone(),
            group_write(
                write_op_with_metadata(additional_type_idx, 100),
                vec![],
                0,
                0,
            ),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 1);
        assert_eq!(
            extract_group_op(base_update.get(&key).unwrap())
                .metadata_op
                .metadata(),
            // take the original metadata
            &raw_metadata(100)
        );
    }

    #[test_case(0, 0)] // create, create
    #[test_case(1, 0)] // modify, create
    #[test_case(2, 1)] // delete, modify
    #[test_case(2, 2)] // delete, delete
    fn test_squash_groups_error(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(&[0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(
            key.clone(),
            group_write(write_op_with_metadata(base_type_idx, 100), vec![], 0, 0),
        );
        additional_update.insert(
            key.clone(),
            group_write(
                write_op_with_metadata(additional_type_idx, 200),
                vec![],
                0,
                0,
            ),
        );

        assert_err!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
    }

    #[test]
    fn test_squash_groups_noop() {
        let key = StateKey::raw(&[0]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        base_update.insert(
            key.clone(),
            group_write(
                write_op_with_metadata(CREATION, 100), // create
                vec![],
                0,
                0,
            ),
        );
        additional_update.insert(
            key.clone(),
            group_write(
                write_op_with_metadata(DELETION, 100), // delete
                vec![],
                0,
                0,
            ),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
        assert!(base_update.is_empty(), "Must become a no-op");
    }

    #[test]
    fn test_inner_ops() {
        let key_1 = StateKey::raw(&[1]);
        let key_2 = StateKey::raw(&[2]);

        let mut base_update = BTreeMap::new();
        let mut additional_update = BTreeMap::new();
        // TODO[agg_v2](test): Hardcoding type layout to None. Test with layout = Some(..)
        base_update.insert(
            key_1.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 100),
                vec![
                    (
                        mock_tag_0(),
                        (WriteOp::legacy_creation(vec![100].into()), None),
                    ),
                    (
                        mock_tag_2(),
                        (WriteOp::legacy_modification(vec![2].into()), None),
                    ),
                ],
                0,
                0,
            ),
        );
        additional_update.insert(
            key_1.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 100),
                vec![
                    (
                        mock_tag_0(),
                        (WriteOp::legacy_modification(vec![0].into()), None),
                    ),
                    (
                        mock_tag_1(),
                        (WriteOp::legacy_modification(vec![1].into()), None),
                    ),
                ],
                0,
                0,
            ),
        );

        base_update.insert(
            key_2.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 100),
                vec![
                    (mock_tag_0(), (WriteOp::legacy_deletion(), None)),
                    (
                        mock_tag_1(),
                        (WriteOp::legacy_modification(vec![2].into()), None),
                    ),
                    (
                        mock_tag_2(),
                        (WriteOp::legacy_creation(vec![2].into()), None),
                    ),
                ],
                0,
                0,
            ),
        );
        additional_update.insert(
            key_2.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 100),
                vec![
                    (
                        mock_tag_0(),
                        (WriteOp::legacy_creation(vec![0].into()), None),
                    ),
                    (mock_tag_1(), (WriteOp::legacy_deletion(), None)),
                    (mock_tag_2(), (WriteOp::legacy_deletion(), None)),
                ],
                0,
                0,
            ),
        );

        assert_ok!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
        assert_eq!(base_update.len(), 2);
        let inner_ops_1 = &extract_group_op(base_update.get(&key_1).unwrap()).inner_ops;
        assert_eq!(inner_ops_1.len(), 3);
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_0()),
            &(WriteOp::legacy_creation(vec![0].into()), None)
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_1()),
            &(WriteOp::legacy_modification(vec![1].into()), None)
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_2()),
            &(WriteOp::legacy_modification(vec![2].into()), None)
        );
        let inner_ops_2 = &extract_group_op(base_update.get(&key_2).unwrap()).inner_ops;
        assert_eq!(inner_ops_2.len(), 2);
        assert_some_eq!(
            inner_ops_2.get(&mock_tag_0()),
            &(WriteOp::legacy_modification(vec![0].into()), None)
        );
        assert_some_eq!(
            inner_ops_2.get(&mock_tag_1()),
            &(WriteOp::legacy_deletion(), None)
        );

        let additional_update = BTreeMap::from([(
            key_2.clone(),
            group_write(
                write_op_with_metadata(MODIFICATION, 100),
                vec![(mock_tag_1(), (WriteOp::legacy_deletion(), None))],
                0,
                0,
            ),
        )]);
        assert_err!(VMChangeSet::squash_additional_resource_writes(
            &mut base_update,
            additional_update
        ));
    }
}
